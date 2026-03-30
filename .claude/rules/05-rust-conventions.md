# Rust conventions for this project

## Edition & Error Handling
- Rust edition 2024
- Use `anyhow::Result` for fallible functions
- Use `anyhow::Context` for adding context to errors — `result.context("failed to load config")?`

## Concurrency Primitives
- Channels: `flume` (not `std::sync::mpsc`)
- Mutexes: `parking_lot::Mutex` (not `std::sync::Mutex`)
- Async PTY reads: `iced::Subscription` (not raw `std::thread::spawn`)

## Serialization
- Config files: `toml` format via `serde` + `toml` crate
- IPC messages: JSON via `serde_json`
- All serializable structs derive `Serialize, Deserialize`

## File Paths
- Use `dirs` crate for `~/.config/jc/`
- Use `std::path::PathBuf` everywhere, never string paths
- Never hardcode `/mnt/c/` or `C:\` — always resolve dynamically

## Dependencies
- Check the workspace `Cargo.toml` before adding a new dependency — it may already be declared there
- Prefer workspace dependencies (`dep.workspace = true`) over per-crate version pinning
