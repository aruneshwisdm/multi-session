# WSL2 development awareness

## File system performance
- Source code must live on ext4 (`~/projects/`) not on Windows mounts (`/mnt/c/`)
- Planning docs and CLAUDE.md live on the Windows mount for easy access — that's fine
- File watchers on `/mnt/c/` paths are unreliable and slow — warn if a user registers a /mnt/c project

## GPU rendering
- wgpu auto-selects backend: D3D12 on native Windows, OpenGL via Mesa on WSL2
- Do not hardcode a wgpu backend — let it auto-detect
- If GPU fails, wgpu falls back to software rendering — this is acceptable

## Notifications
- D-Bus may not be running in WSL2 by default
- Notification code must handle D-Bus unavailability gracefully (log warning, don't crash)

## Testing
- Always test with color terminal commands (`ls --color`, `htop`) to verify VTE compatibility
- Test at multiple DPI scales — WSLg has known font rendering issues at high scaling
- Verify `claude` CLI is in PATH from within WSL2
