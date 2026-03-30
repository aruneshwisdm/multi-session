use std::path::PathBuf;
use std::sync::Mutex;

use crate::ipc;

/// Paths to clean up hooks from on shutdown.
static CLEANUP_PATHS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Install a cross-platform signal handler (SIGINT/SIGTERM on Unix, Ctrl+C/Ctrl+Break on Windows)
/// that cleans up hooks and IPC socket before exiting.
pub fn install_handler(project_paths: &[PathBuf]) {
    *CLEANUP_PATHS.lock().unwrap() = project_paths.to_vec();

    ctrlc::set_handler(move || {
        ipc::cleanup_socket();

        if let Ok(paths) = CLEANUP_PATHS.lock() {
            for path in paths.iter() {
                let _ = jc_core::hooks_settings::uninstall_hooks(path);
            }
        }

        // Exit the process. On Unix, the default SIGINT behavior is to terminate.
        // ctrlc doesn't re-raise the signal, so we exit explicitly.
        std::process::exit(130); // 128 + SIGINT(2)
    })
    .expect("failed to install signal handler");
}
