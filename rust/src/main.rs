fn main() {
    let args: Vec<_> = std::env::args().collect();
    let manager = args.first().is_some_and(|path| {
        std::path::Path::new(path).file_name() == Some(std::ffi::OsStr::new("ai"))
    });
    std::process::exit(if manager {
        aiusage::manager::main()
    } else {
        aiusage::cli::main(args.into_iter().skip(1).collect())
    });
}
