mod app;
mod config;
mod dock;
mod hooks;
mod init;
mod reaper;
mod session;
mod terminal;
mod types;
mod ui;
mod usage;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::load_config;
use crate::dock::run_dock;
use crate::hooks::handle_hook;
use crate::reaper::reap_orphans;
use crate::session::load_sessions_data;
use crate::terminal::create_backend;
use crate::ui::run_tui;

#[derive(Parser)]
#[command(name = "wez-sidebar")]
#[command(about = "WezTerm sidebar for Claude Code session monitoring")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Handle Claude Code hook event
    Hook {
        /// Event name (PreToolUse, PostToolUse, Notification, Stop, UserPromptSubmit)
        event: String,
    },
    /// Run as horizontal dock (bottom bar mode)
    Dock,
    /// Interactive setup wizard
    Init,
    /// Print diagnostic info for debugging
    Diag,
    /// Clean up orphaned Claude Code processes
    Reap {
        /// Dry run: list orphans without killing
        #[arg(long)]
        dry: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config();

    match cli.command {
        Some(Commands::Hook { event }) => {
            handle_hook(&event, &config)?;
        }
        Some(Commands::Dock) => {
            run_dock(config)?;
        }
        Some(Commands::Init) => {
            init::run_init();
        }
        Some(Commands::Diag) => {
            let backend = create_backend(&config.backend, config.effective_terminal_path());
            let pane_id = backend.current_pane_id();
            println!("backend: {}", backend.name());
            println!("current_pane_id: {}", pane_id);

            let panes = backend.list_panes();
            println!("terminal panes: {} found", panes.len());
            for p in &panes {
                let marker = if p.pane_id == pane_id { " <-- self" } else { "" };
                println!("  pane={} tab={} win={} tty={} active={}{}", p.pane_id, p.tab_id, p.window_id, p.tty_name, p.is_active, marker);
            }

            let sessions = load_sessions_data(&config, backend.as_ref());
            println!("\nloaded sessions: {}", sessions.len());
            for s in &sessions {
                println!("  {} tab={} pane={} status={} dc={} stale={}", s.name, s.tab_id, s.pane_id, s.status, s.is_disconnected, s.is_stale);
            }
        }
        Some(Commands::Reap { dry }) => {
            let label = if dry { "[DRY RUN] " } else { "" };
            let reaped = reap_orphans(&config, dry);
            if reaped.is_empty() {
                println!("{}No orphaned Claude Code processes found.", label);
            } else {
                println!("{}Found {} orphan(s):", label, reaped.len());
                for p in &reaped {
                    let action = if dry { "would kill" } else { "killed" };
                    println!(
                        "  {} PID={} PGID={} TTY={} elapsed={} {}",
                        action, p.pid, p.pgid, p.tty, p.elapsed, p.args
                    );
                }
            }
        }
        None => {
            run_tui(config)?;
        }
    }

    Ok(())
}
