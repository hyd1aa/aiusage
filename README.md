# aiusage

`aiusage` is a terminal dashboard for verified AI usage-limit data. Real mode
never invents quota values. Providers without a reliable local adapter are
shown with an explicit availability state when enabled.

```console
aiusage
aiusage --demo
```

Demo mode is visibly labelled and uses local fixtures only. It never reads
credentials, local provider sessions, or remote usage APIs.

Keys: `L` language, `P` position, `S` providers, `R` refresh, and `Q`, `Esc`,
or `Ctrl+C` to exit. Settings are stored in
`~/.config/aiusage/config.toml`.

