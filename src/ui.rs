use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use std::sync::Arc;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Terminal,
};

#[derive(Debug, Clone)]
pub struct ChainUi {
    pub enabled: bool,
    pub pools: usize,

    // HyperIndex “catalog load” progress
    pub hyperindex_target: usize,
    pub hyperindex_loaded: usize,
    pub hyperindex_percent: f64,
    pub hyperindex_status: String, // "starting", "loading", "ready", "error"

    // latest known block (optional)
    pub last_block: Option<u64>,

    // last profitable opportunity (optional)
    pub last_profit_usd: Option<f64>,

    pub last_update: Instant,
}

impl ChainUi {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            pools: 0,
            hyperindex_target: 0,
            hyperindex_loaded: 0,
            hyperindex_percent: 0.0,
            hyperindex_status: "starting".to_string(),
            last_block: None,
            last_profit_usd: None,
            last_update: Instant::now(),
        }
    }
}

#[derive(Debug, Default)]
pub struct UiState {
    pub chains: BTreeMap<String, ChainUi>,
    pub status_line: String,
}

#[derive(Clone)]
pub struct UiHandle {
    inner: Arc<RwLock<UiState>>,
}

impl UiHandle {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(UiState::default())),
        }
    }

    pub fn state(&self) -> Arc<RwLock<UiState>> {
        self.inner.clone()
    }

    pub async fn init_chain(&self, name: &str, enabled: bool, hyperindex_target: usize) {
        let mut st = self.inner.write().await;
        let mut c = ChainUi::new(enabled);
        c.hyperindex_target = hyperindex_target;
        st.chains.insert(name.to_string(), c);
    }

    pub async fn set_status(&self, s: impl Into<String>) {
        let mut st = self.inner.write().await;
        st.status_line = s.into();
    }

    pub async fn set_pools(&self, chain: &str, pools: usize) {
        let mut st = self.inner.write().await;
        if let Some(c) = st.chains.get_mut(chain) {
            c.pools = pools;
            c.last_update = Instant::now();
        }
    }

    pub async fn set_hyperindex_progress(&self, chain: &str, loaded: usize, status: &str) {
        let mut st = self.inner.write().await;
        if let Some(c) = st.chains.get_mut(chain) {
            c.hyperindex_loaded = loaded;
            c.hyperindex_status = status.to_string();
            if c.hyperindex_target > 0 {
                c.hyperindex_percent =
                    (loaded as f64 / c.hyperindex_target as f64).min(1.0) * 100.0;
            } else {
                c.hyperindex_percent = 0.0;
            }
            c.last_update = Instant::now();
        }
    }

    pub async fn set_last_block(&self, chain: &str, bn: u64) {
        let mut st = self.inner.write().await;
        if let Some(c) = st.chains.get_mut(chain) {
            c.last_block = Some(bn);
            c.last_update = Instant::now();
        }
    }

    pub async fn set_last_profit(&self, chain: &str, profit_usd: f64) {
        let mut st = self.inner.write().await;
        if let Some(c) = st.chains.get_mut(chain) {
            c.last_profit_usd = Some(profit_usd);
            c.last_update = Instant::now();
        }
    }
}

/// Run the TUI until user presses 'q' or Ctrl+C terminates the process.
pub async fn run_tui(handle: UiHandle) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let tick_rate = Duration::from_millis(120);

    loop {
        // Input
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(k) = event::read()? {
                if k.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        // Render
        let st = handle.state();
        let snapshot = { st.read().await.clone() };

        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // header
                    Constraint::Length(3),  // global progress
                    Constraint::Min(8),     // table
                    Constraint::Length(3),  // footer
                ])
                .split(size);

            render_header(f, chunks[0], &snapshot.status_line);
            render_global_progress(f, chunks[1], &snapshot);
            render_table(f, chunks[2], &snapshot);
            render_footer(f, chunks[3]);
        })?;

        tokio::time::sleep(tick_rate).await;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_header(f: &mut ratatui::Frame, area: Rect, status: &str) {
    let title = Line::from(vec![
        Span::styled("arbitrage-engine", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::raw(status),
    ]);

    let p = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(p, area);
}

fn render_global_progress(f: &mut ratatui::Frame, area: Rect, st: &UiState) {
    // global hyperindex progress = average of enabled chains with target>0
    let mut sum = 0.0;
    let mut n = 0.0;
    for (_name, c) in &st.chains {
        if c.enabled && c.hyperindex_target > 0 {
            sum += c.hyperindex_percent;
            n += 1.0;
        }
    }
    let avg = if n > 0.0 { sum / n } else { 0.0 };

    let g = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("HyperIndex catalog progress (avg)"))
        .gauge_style(Style::default().add_modifier(Modifier::BOLD))
        .percent(avg.round() as u16);
    f.render_widget(g, area);
}

fn render_table(f: &mut ratatui::Frame, area: Rect, st: &UiState) {
    let header = Row::new(vec![
        "chain", "enabled", "pools", "hyperindex", "target", "loaded", "last_block", "last_profit_usd",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = st.chains.iter().map(|(name, c)| {
        let hi = format!("{} {:>5.1}%", c.hyperindex_status, c.hyperindex_percent);
        Row::new(vec![
            name.clone(),
            if c.enabled { "yes".into() } else { "no".into() },
            c.pools.to_string(),
            hi,
            c.hyperindex_target.to_string(),
            c.hyperindex_loaded.to_string(),
            c.last_block.map(|x| x.to_string()).unwrap_or_else(|| "-".into()),
            c.last_profit_usd
                .map(|x| format!("{:.4}", x))
                .unwrap_or_else(|| "-".into()),
        ])
    });

    let t = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(22),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(16),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Chains"));

    f.render_widget(t, area);
}

fn render_footer(f: &mut ratatui::Frame, area: Rect) {
    let p = Paragraph::new(Line::from(vec![
        Span::raw("Keys: "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to quit TUI (process continues only if you run under supervisor). "),
        Span::raw("Use "),
        Span::styled("TUI=1", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to enable."),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(p, area);
}
