# Upstream sync protocol

The upstream repo is `jeapostrophe/jc` (macOS-only, GPUI). We maintain a fork.

## What to sync
- `jc-core/` — cherry-pick directly, we don't modify this crate
- New hook event types, problem types, TODO format changes — apply to our jc-core
- New features (views, keybindings) — port the logic manually into our iced implementation

## What NOT to sync
- `gpui` dependency changes
- `objc2` / macOS notification changes
- `vendor/gpui-component` changes
- `Info.plist`, `make.sh`, bundle scripts

## How to sync
```bash
git remote add upstream https://github.com/jeapostrophe/jc.git
git fetch upstream
# For jc-core changes:
git checkout upstream/master -- jc-core/
# Review and commit
```

## Commit convention
- Prefix upstream syncs with `upstream:` — e.g., `upstream: sync jc-core with latest`
- Prefix port work with `port:` — e.g., `port: rewrite terminal view for iced`
- Prefix platform work with `platform:` — e.g., `platform: add Windows named pipe IPC`
