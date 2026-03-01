mod api_client;
mod app;
mod config;
mod dock;
mod session;
mod tasks;
mod types;
mod ui;
mod usage;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::load_config;
use crate::dock::run_dock;
use crate::session::{get_wezterm_panes, load_sessions_data};
use crate::ui::run_tui;

#[derive(Parser)]
#[command(name = "wez-sidebar")]
#[command(about = "WezTerm sidebar with Claude Code monitoring and task management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as horizontal dock (bottom bar mode)
    Dock,
    /// Print diagnostic info for debugging
    Diag,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config();

    match cli.command {
        Some(Commands::Dock) => {
            run_dock(config)?;
        }
        Some(Commands::Diag) => {
            let pane_env = std::env::var("WEZTERM_PANE").unwrap_or_else(|_| "(not set)".into());
            println!("WEZTERM_PANE: {}", pane_env);
            println!("wezterm_path: {}", config.wezterm_path);

            let panes = get_wezterm_panes(&config.wezterm_path);
            println!("wezterm panes: {} found", panes.len());
            for p in &panes {
                let marker = if p.pane_id.to_string() == pane_env { " <-- self" } else { "" };
                println!("  pane={} tab={} win={} tty={} active={}{}", p.pane_id, p.tab_id, p.window_id, p.tty_name, p.is_active, marker);
            }

            let sessions = load_sessions_data(&config);
            println!("\nloaded sessions: {}", sessions.len());
            for s in &sessions {
                println!("  {} tab={} pane={} status={} dc={} stale={}", s.name, s.tab_id, s.pane_id, s.status, s.is_disconnected, s.is_stale);
            }
        }
        None => {
            run_tui(config)?;
        }
    }

    Ok(())
}
