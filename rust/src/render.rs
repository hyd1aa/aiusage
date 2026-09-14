//! Pure frame assembly; explicit clock makes byte-level compatibility testable.
use chrono::Datelike;
use unicode_width::UnicodeWidthChar;
use crate::{config::Config, i18n::tr, models::{Availability, ProviderUsage}, timezones, PROVIDERS};

pub fn char_width(c:char)->usize {c.width().unwrap_or(0)}
pub fn visible_len(text:&str)->usize {
    let mut chars=text.chars().peekable();
    let mut count=0;
    while let Some(c)=chars.next() {
        if c=='\x1b' && chars.peek()==Some(&'[') {
            let saved=chars.clone(); chars.next();
            while chars.peek().is_some_and(|c|c.is_ascii_digit()||*c==';') {chars.next();}
            if chars.peek()==Some(&'m') {chars.next();continue;}
            chars=saved;
        }
        count+=char_width(c);
    }
    count
}
pub fn fit(text:&str,width:usize)->String {
    let mut used=0;
    text.chars().take_while(|c| {used+=char_width(*c);used<=width}).collect()
}
pub fn pad(text:&str,width:usize)->String {
    let value=fit(text,width);
    format!("{value}{}"," ".repeat(width.saturating_sub(visible_len(&value))))
}
pub fn label_value(label:&str,value:&str,language:&str)->String {
    format!("{label}{}{value}",if language=="zh" {"："} else {": "})
}
pub fn reset_text(epoch:Option<f64>,language:&str,timezone:&str)->String {
    let Some(value)=epoch.and_then(|e|timezones::from_epoch(e,timezone).ok()) else {return "?".into()};
    let stamp=if language=="zh" {format!("{}月{:02}日 {}",value.month(),value.day(),value.format("%H:%M"))}
        else {value.format("%b %d %H:%M").to_string()};
    format!("{stamp} {}",timezones::label_for(&value))
}
pub fn system_text(language:&str,timezone:&str,now:f64)->String {
    let value=timezones::from_epoch(now,timezone).expect("validated timezone/clock");
    label_value(tr(language,"system"),&format!("{} {}",value.format("%Y-%m-%d %H:%M:%S"),timezones::label_for(&value)),language)
}
pub fn column_count(count:usize,width:usize)->usize {
    if count<=2 {return 1;}
    let capacity=((width+2)/26).clamp(1,3);
    if count==4 && capacity>=2 {2} else {count.min(capacity)}
}
fn provider_lines(provider:&ProviderUsage,width:usize,language:&str,timezone:&str)->Vec<String> {
    let mut lines=vec![format!("{}{}",provider.name.to_uppercase(),if provider.stale {format!("  ! {}",tr(language,"stale"))} else {String::new()})];
    if provider.availability!=Availability::Available || provider.windows.is_empty() {
        let key=match provider.availability {Availability::Available=>"available",Availability::NotInstalled=>"not_installed",Availability::Unavailable=>"unavailable",Availability::NotSupported=>"not_supported"};
        lines.push(tr(language,key).into());
    } else {
        let bar_width=width.saturating_sub(17).clamp(3,10);
        for window in provider.windows.iter().take(2) {
            let remaining=window.remaining_percent.clamp(0,100);
            let fill=(remaining as f64*bar_width as f64/100.0).round_ties_even() as usize;
            let label=if window.label=="Week" && language=="en" {"Weekly"} else {&window.label};
            let label=fit(label,6);
            // Python's <6 pads by codepoints here, not terminal cells.
            let label=format!("{label}{}"," ".repeat(6usize.saturating_sub(label.chars().count())));
            lines.push(format!("{label} {}{} {remaining:>3}% {}","█".repeat(fill),"░".repeat(bar_width-fill),tr(language,"left")));
            let value=reset_text(window.reset_at,language,timezone);
            let full=label_value(tr(language,"reset"),&value,language);
            lines.push(if visible_len(&full)<=width {full} else {value});
        }
    }
    lines.into_iter().map(|s|fit(&s,width)).collect()
}
fn top_border(width:usize,title:&str)->String {
    let inner=width.saturating_sub(2);
    let token=fit(&format!(" {title} "),inner);
    let remaining=inner.saturating_sub(visible_len(&token));
    format!("┌{}{token}{}{}","─".repeat(remaining/2),"─".repeat(remaining-remaining/2),if width>1 {"┐"} else {""})
}
fn outer_line(text:&str,width:usize)->String {
    if width<2 {fit(text,width)} else {format!("│{}│",pad(text,width-2))}
}
fn position(lines:Vec<String>,width:usize,height:usize,pos:&str)->Vec<String> {
    let block_width=lines.iter().map(|s|visible_len(s)).max().unwrap_or(0);
    let (vertical,horizontal)=if pos=="center" {("center","center")} else {pos.split_once('-').unwrap_or(("center","center"))};
    let left=match horizontal {"left"=>0,"right"=>width.saturating_sub(block_width),_=>width.saturating_sub(block_width)/2};
    let top=match vertical {"top"=>0,"bottom"=>height.saturating_sub(lines.len()),_=>height.saturating_sub(lines.len())/2};
    let mut result=vec![String::new();top];
    result.extend(lines.iter().map(|line|fit(&format!("{}{line}"," ".repeat(left)),width)));
    result
}
fn style(lines:&mut [String],kinds:&[&str],theme:&str,color:bool) {
    if !color {return;}
    for (line,kind) in lines.iter_mut().zip(kinds) {
        if line.is_empty() {continue;}
        let code=match (theme,*kind) {("green","strong")=>"1;92",("green",_)=>"32",(_,"strong")=>"1;97",(_,"muted")=>"90",_=>"37"};
        *line=format!("\x1b[{code}m{line}\x1b[0m");
    }
}

pub struct Frame<'a> {
    pub width:usize, pub height:usize, pub providers:&'a [ProviderUsage],
    pub updated:Option<f64>, pub now:f64, pub config:&'a Config,
    pub demo:bool, pub color:bool, pub notice:Option<&'a str>,
}
pub fn dashboard(frame:Frame<'_>)->Vec<String> {
    let Frame{width,height,providers,updated,now,config:cfg,demo,color,notice}=frame;
    let (width,height)=(width.max(1),height.max(1));
    let language=&cfg.language;
    let title=if demo {format!("AI USAGE [{}]",tr(language,"demo"))} else {"AI USAGE".into()};
    if width<4 || height<3 {return position(vec![fit(&title,width)],width,height,&cfg.position);}
    let stamp=updated.and_then(|e|timezones::from_epoch(e,&cfg.timezone).ok()).map(|v|v.format("%H:%M:%S").to_string()).unwrap_or("--:--:--".into());
    let update=label_value(tr(language,"updated"),&stamp,language);
    let system=system_text(language,&cfg.timezone,now);
    let capacity=width.saturating_sub(4).max(1);
    let columns=column_count(providers.len(),capacity);
    let preferred=[0,40,27,24][columns];
    let cell=preferred.min(capacity.saturating_sub(2*(columns-1)).checked_div(columns).unwrap().max(1));
    let grid=cell*columns+2*(columns-1);
    let outer=width.min((grid+4).max(visible_len(tr(language,"help_compact"))+4).max(visible_len(&system)+4));
    let inner=outer-2;
    let content_width=inner.saturating_sub(2).max(1);
    let cell=cell.min((content_width.saturating_sub(2*(columns-1))/columns).max(1));
    let blocks:Vec<_>=providers.iter().map(|p|provider_lines(p,cell,language,&cfg.timezone)).collect();
    let mut content=vec![String::new()];
    let mut kinds=vec!["normal"];
    for (row_index,group) in blocks.chunks(columns).enumerate() {
        let row_height=group.iter().map(Vec::len).max().unwrap_or(1);
        let group_width=group.len()*cell+group.len().saturating_sub(1)*2;
        let left=content_width.saturating_sub(group_width)/2;
        for index in 0..row_height {
            let value=group.iter().map(|b|pad(b.get(index).map(String::as_str).unwrap_or(""),cell)).collect::<Vec<_>>().join("  ");
            content.push(format!(" {}{value}"," ".repeat(left)));
            kinds.push(if index==0 {"strong"} else {"normal"});
        }
        if (row_index+1)*columns<blocks.len() {content.push(String::new());kinds.push("normal");}
    }
    let help=tr(language,if visible_len(tr(language,"help"))<=content_width.saturating_sub(1) {"help"} else {"help_compact"});
    content.extend([String::new(),format!(" {system}"),format!(" {update}")]);
    kinds.extend(["normal","normal","muted"]);
    if let Some(notice)=notice {content.push(format!(" {notice}"));kinds.push("strong");}
    content.extend([String::new(),format!(" {help}"),String::new()]);kinds.extend(["normal","muted","normal"]);
    content.truncate(height-2);kinds.truncate(height-2);
    let mut raw=vec![top_border(outer,&title)];
    raw.extend(content.iter().map(|s|outer_line(&fit(s,inner),outer)));
    raw.push(format!("└{}┘","─".repeat(outer-2)));
    kinds.insert(0,"strong");kinds.push("normal");
    raw.truncate(height);
    let count=raw.len();
    let mut positioned=position(raw,width,height,&cfg.position);
    let top=positioned.len()-count;
    style(&mut positioned[top..],&kinds,&cfg.theme,color);
    positioned
}

pub fn selector(width:usize,height:usize,enabled:&[String],cursor:usize,cfg:&Config,color:bool,discovery:&std::collections::HashMap<String,String>)->Vec<String> {
    let box_width=width.saturating_sub(4).clamp(30,58);
    let inner=box_width-2;
    let row=|s:&str|format!("│ {} │",pad(s,inner-2));
    let mut lines=vec![format!("┌{}┐","─".repeat(inner)),row(tr(&cfg.language,"providers"))];
    for (index,(key,name)) in PROVIDERS.iter().enumerate() {
        let order=enabled.iter().position(|k|k==key).map(|n|n+1);
        let mut value=format!("{} [{}] {name}{}",if index==cursor {"›"} else {" "},if order.is_some() {"x"} else {" "},order.map(|n|format!("  {n}")).unwrap_or_default());
        if let Some(reason)=discovery.get(*key) {
            let available=(inner-3).saturating_sub(visible_len(&value));
            if available>=5 {value.push_str(&format!("  {}",fit(tr(&cfg.language,reason),available-2)));}
        }
        lines.push(row(&value));
    }
    lines.push(row(&fit(tr(&cfg.language,"select_help"),inner-2)));
    lines.push(format!("└{}┘","─".repeat(inner)));
    selection_position(lines,width,height,cfg,color)
}
fn selection_position(lines:Vec<String>,width:usize,height:usize,cfg:&Config,color:bool)->Vec<String> {
    let count=lines.len();let mut lines=position(lines,width,height,"center");let top=lines.len()-count;
    style(&mut lines[top..],&vec!["normal";count],&cfg.theme,color);lines
}
pub fn timezone_selector(width:usize,height:usize,options:&[String],cursor:usize,cfg:&Config,color:bool)->Vec<String> {
    let box_width=width.saturating_sub(4).clamp(34,60);let inner=box_width-2;
    let row=|s:&str|format!("│ {} │",pad(s,inner-2));
    let mut lines=vec![top_border(box_width,tr(&cfg.language,"timezone"))];
    for (index,value) in options.iter().enumerate() {
        let label=if value=="system" {tr(&cfg.language,"system_zone").into()}
            else if index==options.len()-1 {format!("{}: {value}",tr(&cfg.language,"custom_zone"))} else {value.clone()};
        lines.push(row(&format!("{} {label}",if index==cursor {"›"} else {" "})));
    }
    lines.push(row(&fit(tr(&cfg.language,"timezone_help"),inner-2)));lines.push(format!("└{}┘","─".repeat(inner)));
    selection_position(lines,width,height,cfg,color)
}
