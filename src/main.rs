mod api;
mod app;
mod config;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(
    name = "obm",
    version,
    about = "TUI for monitoring OpenAI Batch API job statuses"
)]
struct Cli {
    /// OpenAI API key (defaults to $OPENAI_API_KEY, then config file)
    #[arg(long)]
    api_key: Option<String>,

    /// Seconds between automatic refreshes
    #[arg(long)]
    refresh: Option<u64>,
}

enum AppMsg {
    Batches(Result<Vec<api::Batch>>),
    Cancelled(Box<Result<api::Batch>>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg = match config::Config::load(cli.api_key, cli.refresh) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("omarchy-batch-monitor: {e:#}");
            eprintln!(
                "\nSet the OPENAI_API_KEY environment variable, pass --api-key, or create a \
                 config file at {} with:\n\n  api_key = \"sk-...\"\n",
                config::Config::example_path()
            );
            std::process::exit(1);
        }
    };

    let client = Arc::new(api::Client::new(
        cfg.api_key.clone(),
        cfg.organization.clone(),
        cfg.project.clone(),
        None,
    )?);

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, client, cfg.refresh_secs).await;
    restore_terminal(&mut terminal)?;

    if let Err(e) = &result {
        eprintln!("omarchy-batch-monitor exited with error: {e:#}");
    }
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn spawn_fetch(client: Arc<api::Client>, tx: mpsc::UnboundedSender<AppMsg>) {
    tokio::spawn(async move {
        let res = client.list_all_batches().await;
        let _ = tx.send(AppMsg::Batches(res));
    });
}

fn spawn_cancel(client: Arc<api::Client>, id: String, tx: mpsc::UnboundedSender<AppMsg>) {
    tokio::spawn(async move {
        let res = client.cancel_batch(&id).await;
        let _ = tx.send(AppMsg::Cancelled(Box::new(res)));
    });
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    client: Arc<api::Client>,
    refresh_secs: u64,
) -> Result<()> {
    let mut app = App::new(refresh_secs);
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMsg>();

    spawn_fetch(client.clone(), tx.clone());

    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_secs(1));

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            _ = ticker.tick() => {
                app.tick_countdown();
                if !app.loading && app.seconds_until_refresh == 0 {
                    app.loading = true;
                    spawn_fetch(client.clone(), tx.clone());
                }
            }
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        handle_key(&mut app, key.code, &client, &tx);
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        app.set_error(format!("input error: {e}"));
                    }
                    None => {
                        app.should_quit = true;
                    }
                }
            }
            msg = rx.recv() => {
                match msg {
                    Some(AppMsg::Batches(Ok(batches))) => app.set_batches(batches),
                    Some(AppMsg::Batches(Err(e))) => app.set_error(format!("{e:#}")),
                    Some(AppMsg::Cancelled(boxed)) => match *boxed {
                        Ok(batch) => {
                            app.flash(format!("cancel requested for {}", batch.id));
                            app.loading = true;
                            spawn_fetch(client.clone(), tx.clone());
                        }
                        Err(e) => app.set_error(format!("{e:#}")),
                    },
                    None => {}
                }
            }
        }
    }

    Ok(())
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    client: &Arc<api::Client>,
    tx: &mpsc::UnboundedSender<AppMsg>,
) {
    if app.show_help {
        app.show_help = false;
        return;
    }

    if app.confirm_cancel {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm_cancel = false;
                if let Some(batch) = app.selected_batch() {
                    spawn_cancel(client.clone(), batch.id.clone(), tx.clone());
                    app.flash("cancelling…");
                }
            }
            _ => {
                app.confirm_cancel = false;
            }
        }
        return;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('g') | KeyCode::Home => app.select_first(),
        KeyCode::Char('G') | KeyCode::End => app.select_last(),
        KeyCode::Char('r') => {
            if !app.loading {
                app.loading = true;
                spawn_fetch(client.clone(), tx.clone());
            }
        }
        KeyCode::Char('s') => app.toggle_sort(),
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('c') => {
            if let Some(batch) = app.selected_batch() {
                if App::is_cancellable(batch) {
                    app.confirm_cancel = true;
                } else {
                    app.flash(format!("batch is {} — not cancellable", batch.status));
                }
            }
        }
        _ => {}
    }
}
