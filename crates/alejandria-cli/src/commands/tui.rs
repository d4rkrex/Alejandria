//! TUI Admin Dashboard for Alejandria
//!
//! Interactive terminal UI for SSH-friendly API key management and system monitoring.
//!
//! Features:
//! - 3 tabs: API Keys, Stats, Activity Log
//! - Split panel: List (left) + Detail (right)
//! - Actions: n=new, r=revoke, R=revoke-user, f=filter, /=search, ?=help, q=quit
//! - Vim keybindings: j/k navigation, gg/G first/last, Enter=select
//! - Color coding: green (active), red (revoked), yellow (expired)
//! - ASCII bar charts for stats

use alejandria_storage::{api_keys, SqliteStore};
use anyhow::{Context, Result};
// use chrono::Utc; // Unused - keeping for future timestamp features
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    ApiKeys,
    Stats,
    ActivityLog,
}

impl Tab {
    fn titles() -> Vec<&'static str> {
        vec!["API Keys", "Stats", "Activity Log"]
    }

    fn next(&self) -> Self {
        match self {
            Tab::ApiKeys => Tab::Stats,
            Tab::Stats => Tab::ActivityLog,
            Tab::ActivityLog => Tab::ApiKeys,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Tab::ApiKeys => Tab::ActivityLog,
            Tab::Stats => Tab::ApiKeys,
            Tab::ActivityLog => Tab::Stats,
        }
    }
}

#[derive(Debug)]
struct AppState {
    current_tab: Tab,
    keys_list_state: ListState,
    keys: Vec<api_keys::ApiKey>,
    selected_key_index: Option<usize>,
    filter_user: Option<String>,
    search_query: Option<String>,
    show_revoked: bool,
    show_help: bool,
    input_mode: InputMode,
    input_buffer: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // NewKey reserved for future interactive key creation
enum InputMode {
    Normal,
    Filter,
    Search,
    NewKey,
}

impl AppState {
    fn new(keys: Vec<api_keys::ApiKey>) -> Self {
        let mut state = ListState::default();
        if !keys.is_empty() {
            state.select(Some(0));
        }

        Self {
            current_tab: Tab::ApiKeys,
            keys_list_state: state,
            keys,
            selected_key_index: Some(0),
            filter_user: None,
            search_query: None,
            show_revoked: false,
            show_help: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
        }
    }

    fn filtered_keys(&self) -> Vec<&api_keys::ApiKey> {
        self.keys
            .iter()
            .filter(|key| {
                // Filter by revoked status
                if !self.show_revoked && key.revoked_at.is_some() {
                    return false;
                }

                // Filter by user
                if let Some(ref user) = self.filter_user {
                    if !key.username.contains(user) {
                        return false;
                    }
                }

                // Filter by search query
                if let Some(ref query) = self.search_query {
                    let query_lower = query.to_lowercase();
                    let matches_username = key.username.to_lowercase().contains(&query_lower);
                    let matches_desc = key
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);
                    let matches_id = key.id.to_lowercase().contains(&query_lower);

                    if !matches_username && !matches_desc && !matches_id {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    fn selected_key(&self) -> Option<&api_keys::ApiKey> {
        let filtered = self.filtered_keys();
        self.selected_key_index
            .and_then(|idx| filtered.get(idx).copied())
    }

    fn next_key(&mut self) {
        let filtered_count = self.filtered_keys().len();
        if filtered_count == 0 {
            return;
        }

        let i = match self.selected_key_index {
            Some(i) => {
                if i >= filtered_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_key_index = Some(i);
        self.keys_list_state.select(Some(i));
    }

    fn prev_key(&mut self) {
        let filtered_count = self.filtered_keys().len();
        if filtered_count == 0 {
            return;
        }

        let i = match self.selected_key_index {
            Some(i) => {
                if i == 0 {
                    filtered_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_key_index = Some(i);
        self.keys_list_state.select(Some(i));
    }

    fn first_key(&mut self) {
        if !self.filtered_keys().is_empty() {
            self.selected_key_index = Some(0);
            self.keys_list_state.select(Some(0));
        }
    }

    fn last_key(&mut self) {
        let filtered_count = self.filtered_keys().len();
        if filtered_count > 0 {
            let last = filtered_count - 1;
            self.selected_key_index = Some(last);
            self.keys_list_state.select(Some(last));
        }
    }
}

/// Run the TUI admin dashboard
pub fn run() -> Result<()> {
    // Load configuration and database
    let config = Config::load()?;
    let db_path = config.expand_db_path()?;
    let store = SqliteStore::open(&db_path).context("Failed to open database")?;

    // Load all API keys
    let keys = store
        .with_conn(|conn| api_keys::list_api_keys(conn, true, true))
        .context("Failed to load API keys")?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = AppState::new(keys);

    // Main loop
    let result = run_app(&mut terminal, &mut app, &store);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    store: &SqliteStore,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => {
                    if app.show_help {
                        // Close help on any key
                        app.show_help = false;
                        continue;
                    }

                    match (key.code, key.modifiers) {
                        // Quit
                        (KeyCode::Char('q'), _) => return Ok(()),
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),

                        // Tab navigation
                        (KeyCode::Tab, _) => app.current_tab = app.current_tab.next(),
                        (KeyCode::BackTab, _) => app.current_tab = app.current_tab.prev(),
                        (KeyCode::Char('1'), _) => app.current_tab = Tab::ApiKeys,
                        (KeyCode::Char('2'), _) => app.current_tab = Tab::Stats,
                        (KeyCode::Char('3'), _) => app.current_tab = Tab::ActivityLog,

                        // Help
                        (KeyCode::Char('?'), _) => app.show_help = true,

                        // API Keys tab navigation (Vim style)
                        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.next_key(),
                        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.prev_key(),
                        (KeyCode::Char('g'), _) => app.first_key(),
                        (KeyCode::Char('G'), KeyModifiers::SHIFT) => app.last_key(),

                        // Actions
                        (KeyCode::Char('r'), _) => {
                            if let Some(key) = app.selected_key() {
                                revoke_key_interactive(store, &key.id)?;
                                reload_keys(app, store)?;
                            }
                        }
                        (KeyCode::Char('R'), KeyModifiers::SHIFT) => {
                            if let Some(key) = app.selected_key() {
                                revoke_user_interactive(store, &key.username)?;
                                reload_keys(app, store)?;
                            }
                        }
                        (KeyCode::Char('t'), _) => {
                            app.show_revoked = !app.show_revoked;
                        }
                        (KeyCode::Char('f'), _) => {
                            app.input_mode = InputMode::Filter;
                            app.input_buffer.clear();
                        }
                        (KeyCode::Char('/'), _) => {
                            app.input_mode = InputMode::Search;
                            app.input_buffer.clear();
                        }
                        (KeyCode::Char('c'), _) => {
                            // Clear filters
                            app.filter_user = None;
                            app.search_query = None;
                        }

                        _ => {}
                    }
                }
                InputMode::Filter | InputMode::Search | InputMode::NewKey => {
                    match key.code {
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Enter => {
                            // Apply filter/search
                            match app.input_mode {
                                InputMode::Filter => {
                                    if app.input_buffer.is_empty() {
                                        app.filter_user = None;
                                    } else {
                                        app.filter_user = Some(app.input_buffer.clone());
                                    }
                                }
                                InputMode::Search => {
                                    if app.input_buffer.is_empty() {
                                        app.search_query = None;
                                    } else {
                                        app.search_query = Some(app.input_buffer.clone());
                                    }
                                }
                                _ => {}
                            }
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                            app.first_key(); // Reset selection
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &AppState) {
    // Main layout: tabs + content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(f.size());

    // Render tabs
    let titles = Tab::titles();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Alejandria Admin"),
        )
        .select(app.current_tab as usize)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    // Render content based on active tab
    match app.current_tab {
        Tab::ApiKeys => render_api_keys_tab(f, app, chunks[1]),
        Tab::Stats => render_stats_tab(f, app, chunks[1]),
        Tab::ActivityLog => render_activity_log_tab(f, app, chunks[1]),
    }

    // Status bar
    render_status_bar(f, app, chunks[2]);

    // Help overlay
    if app.show_help {
        render_help_overlay(f);
    }
}

fn render_api_keys_tab(f: &mut Frame, app: &AppState, area: Rect) {
    // Split into list (left) and detail (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Render keys list
    let filtered_keys = app.filtered_keys();
    let items: Vec<ListItem> = filtered_keys
        .iter()
        .map(|key| {
            let status = key.status();
            let (icon, color) = match status {
                "active" => ("✅", Color::Green),
                "revoked" => ("🚫", Color::Red),
                "expired" => ("⏰", Color::Yellow),
                _ => ("❓", Color::Gray),
            };

            let line = Line::from(vec![
                Span::raw(icon),
                Span::raw(" "),
                Span::styled(&key.username, Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(
                    format!("({})", &key.id[..8]),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("API Keys ({})", filtered_keys.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut app.keys_list_state.clone());

    // Render key detail
    if let Some(key) = app.selected_key() {
        render_key_detail(f, key, chunks[1]);
    } else {
        let block = Block::default().borders(Borders::ALL).title("Key Detail");
        let text = Paragraph::new("No key selected").block(block);
        f.render_widget(text, chunks[1]);
    }
}

fn render_key_detail(f: &mut Frame, key: &api_keys::ApiKey, area: Rect) {
    let status = key.status();
    let (status_icon, status_color) = match status {
        "active" => ("✅ ACTIVE", Color::Green),
        "revoked" => ("🚫 REVOKED", Color::Red),
        "expired" => ("⏰ EXPIRED", Color::Yellow),
        _ => ("❓ UNKNOWN", Color::Gray),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(status_icon, Style::default().fg(status_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&key.id),
        ]),
        Line::from(vec![
            Span::styled("Username: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&key.username),
        ]),
    ];

    if let Some(desc) = &key.description {
        lines.push(Line::from(vec![
            Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(desc),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(key.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Created By: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(&key.created_by),
    ]));

    if let Some(expires) = key.expires_at {
        lines.push(Line::from(vec![
            Span::styled("Expires: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(expires.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Expires: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Never"),
        ]));
    }

    if let Some(last_used) = key.last_used_at {
        lines.push(Line::from(vec![
            Span::styled("Last Used: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(last_used.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled(
            "Usage Count: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(key.usage_count.to_string()),
    ]));

    if let Some(revoked) = key.revoked_at {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Revoked At: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(revoked.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Key Detail"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_stats_tab(f: &mut Frame, app: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // Calculate stats
    let total_keys = app.keys.len();
    let active_keys = app.keys.iter().filter(|k| k.status() == "active").count();
    let revoked_keys = app.keys.iter().filter(|k| k.status() == "revoked").count();
    let expired_keys = app.keys.iter().filter(|k| k.status() == "expired").count();

    // Bar chart data
    let data = vec![
        ("Active", active_keys as u64),
        ("Revoked", revoked_keys as u64),
        ("Expired", expired_keys as u64),
    ];

    let bars: Vec<Bar> = data
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let color = match i {
                0 => Color::Green,
                1 => Color::Red,
                2 => Color::Yellow,
                _ => Color::White,
            };
            Bar::default()
                .label(Line::from(*label))
                .value(*value)
                .style(Style::default().fg(color))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("API Key Statistics (Total: {})", total_keys)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(15)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::White))
        .value_style(Style::default().fg(Color::Black).bg(Color::White));

    f.render_widget(chart, chunks[0]);

    // Detailed stats text
    let stats_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Total Keys: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(total_keys.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Active Keys: ", Style::default().fg(Color::Green)),
            Span::raw(active_keys.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Revoked Keys: ", Style::default().fg(Color::Red)),
            Span::raw(revoked_keys.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Expired Keys: ", Style::default().fg(Color::Yellow)),
            Span::raw(expired_keys.to_string()),
        ]),
    ];

    let stats_text =
        Paragraph::new(stats_lines).block(Block::default().borders(Borders::ALL).title("Details"));

    f.render_widget(stats_text, chunks[1]);
}

fn render_activity_log_tab(f: &mut Frame, app: &AppState, area: Rect) {
    // Get recent activity (keys sorted by last_used_at or created_at)
    let mut recent_keys = app.keys.clone();
    recent_keys.sort_by(|a, b| {
        let a_time = a.last_used_at.unwrap_or(a.created_at);
        let b_time = b.last_used_at.unwrap_or(b.created_at);
        b_time.cmp(&a_time) // Reverse order (most recent first)
    });

    let items: Vec<ListItem> = recent_keys
        .iter()
        .take(50) // Show last 50 activities
        .map(|key| {
            let time = key.last_used_at.unwrap_or(key.created_at);
            let action = if key.last_used_at.is_some() {
                "Used"
            } else {
                "Created"
            };

            let line = Line::from(vec![
                Span::styled(
                    time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::styled(action, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::raw(&key.username),
                Span::raw(" "),
                Span::styled(
                    format!("({})", &key.id[..8]),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Activity (Last 50)"),
    );

    f.render_widget(list, area);
}

fn render_status_bar(f: &mut Frame, app: &AppState, area: Rect) {
    let status_text = match app.input_mode {
        InputMode::Normal => {
            let mut parts = vec![Span::raw("q:Quit | ?:Help | Tab:Switch | j/k:Nav")];

            if app.current_tab == Tab::ApiKeys {
                parts.push(Span::raw(
                    " | r:Revoke | R:RevokeUser | f:Filter | /:Search | t:Toggle | c:Clear",
                ));
            }

            if let Some(ref user) = app.filter_user {
                parts.push(Span::raw(format!(" | Filter: {}", user)));
            }

            if let Some(ref query) = app.search_query {
                parts.push(Span::raw(format!(" | Search: {}", query)));
            }

            if app.show_revoked {
                parts.push(Span::raw(" | [Showing Revoked]"));
            }

            parts
        }
        InputMode::Filter => vec![
            Span::styled("Filter by user: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled(
                " (Enter to apply, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        InputMode::Search => vec![
            Span::styled("Search: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled(
                " (Enter to apply, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        InputMode::NewKey => vec![
            Span::styled("New key user: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled(
                " (Enter to continue, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ],
    };

    let status =
        Paragraph::new(Line::from(status_text)).block(Block::default().borders(Borders::ALL));

    f.render_widget(status, area);
}

fn render_help_overlay(f: &mut Frame) {
    let area = f.size();
    let help_width = 60;
    let help_height = 20;
    let help_x = (area.width.saturating_sub(help_width)) / 2;
    let help_y = (area.height.saturating_sub(help_height)) / 2;

    let help_area = Rect::new(help_x, help_y, help_width, help_height);

    let help_lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  Tab / Shift+Tab - Switch tabs"),
        Line::from("  1/2/3 - Jump to tab"),
        Line::from("  j / ↓ - Move down"),
        Line::from("  k / ↑ - Move up"),
        Line::from("  gg - Go to first"),
        Line::from("  G - Go to last"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  r - Revoke selected key"),
        Line::from("  R - Revoke all keys for user"),
        Line::from("  f - Filter by user"),
        Line::from("  / - Search"),
        Line::from("  c - Clear filters"),
        Line::from("  t - Toggle showing revoked keys"),
        Line::from("  ? - Show this help"),
        Line::from("  q - Quit"),
    ];

    let help_text = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(help_text, help_area);
}

fn revoke_key_interactive(store: &SqliteStore, key_id: &str) -> Result<()> {
    store
        .with_conn(|conn| api_keys::revoke_api_key_by_id(conn, key_id))
        .context("Failed to revoke key")?;
    Ok(())
}

fn revoke_user_interactive(store: &SqliteStore, user_id: &str) -> Result<()> {
    store
        .with_conn(|conn| api_keys::revoke_api_keys_for_user(conn, user_id))
        .context("Failed to revoke user keys")?;
    Ok(())
}

fn reload_keys(app: &mut AppState, store: &SqliteStore) -> Result<()> {
    let keys = store
        .with_conn(|conn| api_keys::list_api_keys(conn, true, true))
        .context("Failed to reload API keys")?;

    app.keys = keys;
    app.first_key(); // Reset selection
    Ok(())
}
