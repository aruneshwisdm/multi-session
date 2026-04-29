mod app;
pub mod app_config;
pub mod session_persistence;
#[allow(dead_code)]
mod language;
#[allow(dead_code)]
mod outline;
#[allow(dead_code)]
mod views;

use clap::{Parser, Subcommand};
use jc_core::config;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jc", about = "Claude Code session orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to a project directory to open or register.
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Remove stale jc hooks from all configured projects.
    CleanHooks,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(Command::CleanHooks) = cli.command {
        return cmd_clean_hooks();
    }

    // Resolve the project path early.
    let project_path =
        cli.path.as_ref().map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()));

    // Try to send to an already-running instance.
    if let Some(path) = &project_path {
        if jc_platform::ipc::try_send_to_running(path) {
            return Ok(());
        }
    }

    let config = config::load_config()?;
    let mut state = config::load_state()?;

    if let Some(path) = &project_path {
        state.register_project(path);
        config::save_state(&state)?;
    } else if state.projects.is_empty() {
        let cwd = std::env::current_dir()?;
        state.register_project(&cwd);
        config::save_state(&state)?;
    }

    // Start IPC server.
    let (ipc_tx, ipc_rx) = flume::unbounded::<PathBuf>();
    let _server = jc_platform::ipc::SocketServer::bind(move |path| {
        let _ = ipc_tx.send(path);
    });

    // Install signal handler.
    let project_paths: Vec<PathBuf> = state.projects.iter().map(|p| p.path.clone()).collect();
    jc_platform::signals::install_handler(&project_paths);

    app::run(state, config, ipc_rx);
    Ok(())
}

fn cmd_clean_hooks() -> anyhow::Result<()> {
    let state = config::load_state()?;

    let mut paths: Vec<PathBuf> = state.projects.iter().map(|p| p.path.clone()).collect();

    if let Ok(cwd) = std::env::current_dir() {
        let cwd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
        if !paths.contains(&cwd) {
            paths.push(cwd);
        }
    }

    for path in &paths {
        match jc_core::hooks_settings::uninstall_hooks(path) {
            Ok(()) => eprintln!("cleaned hooks for {}", path.display()),
            Err(e) => eprintln!("failed to clean hooks for {}: {e}", path.display()),
        }
    }
    Ok(())
}
