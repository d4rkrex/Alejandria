//! TUI Admin Dashboard for Alejandria
//!
//! Interactive terminal UI for SSH-friendly API key management and system monitoring.
//!
//! Features:
//! - 6 tabs: API Keys, Stats, Activity Log, Memories, Backup, Help
//! - Split panel: List (left) + Detail (right)
//! - Actions: n=new, r=revoke, R=revoke-user, f=filter, /=search, e=export, d=delete, ?=help, q=quit
//! - Vim keybindings: j/k navigation, gg/G first/last, Enter=select
//! - Color coding: green (active), red (revoked), yellow (expired)
//! - ASCII bar charts for stats
//! - Memory management with search, filter, and export
//! - Backup/restore with export/import wizards

use alejandria_storage::{api_keys, ExportFormat, Memory, MemoryStore, SqliteStore};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
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
use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    ApiKeys,
    Stats,
    ActivityLog,
    Memories,
    Backup,
    Help,
}

impl Tab {
    fn titles() -> Vec<&'static str> {
        vec![
            "API Keys",
            "Stats",
            "Activity Log",
            "Memories",
            "Backup",
            "Help",
        ]
    }

    fn next(&self) -> Self {
        match self {
            Tab::ApiKeys => Tab::Stats,
            Tab::Stats => Tab::ActivityLog,
            Tab::ActivityLog => Tab::Memories,
            Tab::Memories => Tab::Backup,
            Tab::Backup => Tab::Help,
            Tab::Help => Tab::ApiKeys,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Tab::ApiKeys => Tab::Help,
            Tab::Stats => Tab::ApiKeys,
            Tab::ActivityLog => Tab::Stats,
            Tab::Memories => Tab::ActivityLog,
            Tab::Backup => Tab::Memories,
            Tab::Help => Tab::Backup,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum ExportStep {
    FormatSelection,
    FilterConfig,
    PathInput,
    Preview,
    Progress,
    Complete,
}

#[derive(Debug, Clone)]
struct ExportFilters {
    date_from: Option<DateTime<Utc>>,
    date_to: Option<DateTime<Utc>>,
    topic_pattern: Option<String>,
    min_importance: Option<String>,
}

impl Default for ExportFilters {
    fn default() -> Self {
        Self {
            date_from: None,
            date_to: None,
            topic_pattern: None,
            min_importance: None,
        }
    }
}

#[derive(Debug, Clone)]
struct ExportWizardState {
    step: ExportStep,
    format: ExportFormat,
    filters: ExportFilters,
    output_path: PathBuf,
    progress: Option<(usize, usize)>, // current, total
}

impl Default for ExportWizardState {
    fn default() -> Self {
        Self {
            step: ExportStep::FormatSelection,
            format: ExportFormat::Json,
            filters: ExportFilters::default(),
            output_path: PathBuf::new(),
            progress: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ImportMode {
    Skip,
    Update,
    Replace,
}

#[derive(Debug, Clone)]
struct ImportWizardState {
    input_path: PathBuf,
    mode: ImportMode,
    dry_run: bool,
    preview_count: usize,
    progress: Option<(usize, usize)>,
}

impl Default for ImportWizardState {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            mode: ImportMode::Skip,
            dry_run: true,
            preview_count: 0,
            progress: None,
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

    // Memories tab state
    memories_list: Vec<Memory>,
    memories_list_state: ListState,
    topics_list: Vec<(String, usize)>, // (topic, count)
    selected_topic_index: Option<usize>,
    memory_search_query: Option<String>,
    memory_filter_importance: Option<String>,
    show_delete_confirmation: bool,
    pagination_offset: usize,
    selected_memory_index: Option<usize>,

    // Backup tab state
    export_wizard_state: ExportWizardState,
    import_wizard_state: ImportWizardState,

    // Help tab state
    help_scroll_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum InputMode {
    Normal,
    Filter,
    Search,
    NewKey,
    MemorySearch,
    ExportPath,
    ImportPath,
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

            // Memories tab state
            memories_list: Vec::new(),
            memories_list_state: ListState::default(),
            topics_list: Vec::new(),
            selected_topic_index: None,
            memory_search_query: None,
            memory_filter_importance: None,
            show_delete_confirmation: false,
            pagination_offset: 0,
            selected_memory_index: None,

            // Backup tab state
            export_wizard_state: ExportWizardState::default(),
            import_wizard_state: ImportWizardState::default(),

            // Help tab state
            help_scroll_offset: 0,
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

    // Memories tab navigation
    fn next_topic(&mut self) {
        let count = self.topics_list.len();
        if count == 0 {
            return;
        }
        let i = match self.selected_topic_index {
            Some(i) if i >= count - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.selected_topic_index = Some(i);
    }

    fn prev_topic(&mut self) {
        let count = self.topics_list.len();
        if count == 0 {
            return;
        }
        let i = match self.selected_topic_index {
            Some(0) => count - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.selected_topic_index = Some(i);
    }

    fn selected_topic(&self) -> Option<&str> {
        self.selected_topic_index
            .and_then(|i| self.topics_list.get(i))
            .map(|(topic, _)| topic.as_str())
    }

    fn filtered_memories(&self) -> Vec<&Memory> {
        self.memories_list
            .iter()
            .filter(|m| {
                // Filter by importance
                if let Some(ref importance) = self.memory_filter_importance {
                    if &m.importance.to_string().to_lowercase() != importance {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    fn selected_memory(&self) -> Option<&Memory> {
        let filtered = self.filtered_memories();
        self.selected_memory_index
            .and_then(|idx| filtered.get(idx).copied())
    }

    fn next_memory(&mut self) {
        let count = self.filtered_memories().len();
        if count == 0 {
            return;
        }
        let i = match self.selected_memory_index {
            Some(i) if i >= count - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.selected_memory_index = Some(i);
        self.memories_list_state.select(Some(i));
    }

    fn prev_memory(&mut self) {
        let count = self.filtered_memories().len();
        if count == 0 {
            return;
        }
        let i = match self.selected_memory_index {
            Some(0) => count - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.selected_memory_index = Some(i);
        self.memories_list_state.select(Some(i));
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

                        // Tab navigation with data loading
                        (KeyCode::Tab, _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = app.current_tab.next();
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::BackTab, _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = app.current_tab.prev();
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('1'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::ApiKeys;
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('2'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::Stats;
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('3'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::ActivityLog;
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('4'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::Memories;
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('5'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::Backup;
                            handle_tab_switch(app, store, prev_tab)?;
                        }
                        (KeyCode::Char('6'), _) => {
                            let prev_tab = app.current_tab;
                            app.current_tab = Tab::Help;
                            handle_tab_switch(app, store, prev_tab)?;
                        }

                        // Help
                        (KeyCode::Char('?'), _) => app.show_help = true,

                        // Navigation (context-aware)
                        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => match app.current_tab {
                            Tab::ApiKeys => app.next_key(),
                            Tab::Memories => {
                                if app.show_delete_confirmation {
                                    // Do nothing during confirmation
                                } else {
                                    app.next_memory();
                                }
                            }
                            _ => {}
                        },
                        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => match app.current_tab {
                            Tab::ApiKeys => app.prev_key(),
                            Tab::Memories => {
                                if app.show_delete_confirmation {
                                    // Do nothing during confirmation
                                } else {
                                    app.prev_memory();
                                }
                            }
                            _ => {}
                        },
                        (KeyCode::Char('g'), _) => {
                            if app.current_tab == Tab::ApiKeys {
                                app.first_key();
                            }
                        }
                        (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                            if app.current_tab == Tab::ApiKeys {
                                app.last_key();
                            }
                        }

                        // Page navigation for Memories tab
                        (KeyCode::PageDown, _) if app.current_tab == Tab::Memories => {
                            app.pagination_offset += 50;
                            // Reload memories with new offset
                            if let Some(topic) = app.selected_topic() {
                                app.memories_list = load_memories_for_topic_with_offset(
                                    store,
                                    topic,
                                    50,
                                    app.pagination_offset,
                                )?;
                            }
                        }
                        (KeyCode::PageUp, _) if app.current_tab == Tab::Memories => {
                            if app.pagination_offset >= 50 {
                                app.pagination_offset -= 50;
                                if let Some(topic) = app.selected_topic() {
                                    app.memories_list = load_memories_for_topic_with_offset(
                                        store,
                                        topic,
                                        50,
                                        app.pagination_offset,
                                    )?;
                                }
                            }
                        }

                        // Delete confirmation handling
                        (KeyCode::Char('y'), _)
                            if app.show_delete_confirmation && app.current_tab == Tab::Memories =>
                        {
                            if let Some(memory) = app.selected_memory() {
                                let memory_id = memory.id.clone();
                                store.delete(&memory_id)?;
                                app.memories_list.retain(|m| m.id != memory_id);
                                if app.selected_memory_index.is_some()
                                    && !app.memories_list.is_empty()
                                {
                                    let new_idx = app
                                        .selected_memory_index
                                        .unwrap()
                                        .min(app.memories_list.len() - 1);
                                    app.selected_memory_index = Some(new_idx);
                                    app.memories_list_state.select(Some(new_idx));
                                }
                            }
                            app.show_delete_confirmation = false;
                        }
                        (KeyCode::Char('n'), _) if app.show_delete_confirmation => {
                            app.show_delete_confirmation = false;
                        }
                        (KeyCode::Esc, _) if app.show_delete_confirmation => {
                            app.show_delete_confirmation = false;
                        }

                        // API Keys tab actions
                        (KeyCode::Char('r'), _) if app.current_tab == Tab::ApiKeys => {
                            if let Some(key) = app.selected_key() {
                                revoke_key_interactive(store, &key.id)?;
                                reload_keys(app, store)?;
                            }
                        }
                        (KeyCode::Char('R'), KeyModifiers::SHIFT)
                            if app.current_tab == Tab::ApiKeys =>
                        {
                            if let Some(key) = app.selected_key() {
                                revoke_user_interactive(store, &key.username)?;
                                reload_keys(app, store)?;
                            }
                        }
                        (KeyCode::Char('t'), _) if app.current_tab == Tab::ApiKeys => {
                            app.show_revoked = !app.show_revoked;
                        }

                        // Filter/Search (context-aware)
                        (KeyCode::Char('f'), _) => match app.current_tab {
                            Tab::ApiKeys => {
                                app.input_mode = InputMode::Filter;
                                app.input_buffer.clear();
                            }
                            Tab::Memories => {
                                // Cycle through importance levels
                                app.memory_filter_importance =
                                    match app.memory_filter_importance.as_deref() {
                                        None => Some("critical".to_string()),
                                        Some("critical") => Some("high".to_string()),
                                        Some("high") => Some("medium".to_string()),
                                        Some("medium") => Some("low".to_string()),
                                        Some("low") => None,
                                        _ => None,
                                    };
                            }
                            _ => {}
                        },
                        (KeyCode::Char('/'), _) => match app.current_tab {
                            Tab::ApiKeys => {
                                app.input_mode = InputMode::Search;
                                app.input_buffer.clear();
                            }
                            Tab::Memories => {
                                app.input_mode = InputMode::MemorySearch;
                                app.input_buffer.clear();
                            }
                            _ => {}
                        },
                        (KeyCode::Char('c'), _) if app.current_tab == Tab::ApiKeys => {
                            // Clear filters
                            app.filter_user = None;
                            app.search_query = None;
                        }

                        // Memories tab actions
                        (KeyCode::Char('e'), _)
                            if app.current_tab == Tab::Memories
                                && !app.show_delete_confirmation =>
                        {
                            if let Some(memory) = app.selected_memory() {
                                let memory_id = memory.id.clone();
                                app.input_mode = InputMode::ExportPath;
                                app.input_buffer = format!("./memory-{}.json", &memory_id[..8]);
                            }
                        }
                        (KeyCode::Char('d'), _)
                            if app.current_tab == Tab::Memories
                                && !app.show_delete_confirmation =>
                        {
                            if app.selected_memory().is_some() {
                                app.show_delete_confirmation = true;
                            }
                        }

                        _ => {}
                    }
                }
                InputMode::Filter
                | InputMode::Search
                | InputMode::NewKey
                | InputMode::MemorySearch
                | InputMode::ExportPath
                | InputMode::ImportPath => {
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
                                InputMode::MemorySearch => {
                                    if app.input_buffer.is_empty() {
                                        app.memory_search_query = None;
                                        // Reload from topic if selected
                                        if let Some(topic) = app.selected_topic() {
                                            app.memories_list =
                                                load_memories_for_topic(store, topic, Some(50))?;
                                        }
                                    } else {
                                        // Execute FTS5 search
                                        app.memory_search_query = Some(app.input_buffer.clone());
                                        app.memories_list =
                                            search_memories(store, &app.input_buffer, 50)?;
                                        if !app.memories_list.is_empty() {
                                            app.selected_memory_index = Some(0);
                                            app.memories_list_state.select(Some(0));
                                        }
                                    }
                                }
                                InputMode::ExportPath => {
                                    if let Some(memory) = app.selected_memory() {
                                        let path = PathBuf::from(&app.input_buffer);
                                        let json = serde_json::to_string_pretty(memory)?;
                                        std::fs::write(&path, json)?;
                                    }
                                }
                                InputMode::ImportPath => {
                                    // Handled in Phase 3
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

    Ok(())
}

// Helper functions for loading memories
fn load_topics(store: &SqliteStore) -> Result<Vec<(String, usize)>> {
    let topics = store
        .list_topics(None, None)
        .context("Failed to load topics")?;

    Ok(topics.into_iter().map(|t| (t.topic, t.count)).collect())
}

fn load_memories_for_topic(
    store: &SqliteStore,
    topic: &str,
    limit: Option<usize>,
) -> Result<Vec<Memory>> {
    store
        .get_by_topic(topic, limit, None)
        .context("Failed to load memories for topic")
}

fn load_memories_for_topic_with_offset(
    store: &SqliteStore,
    topic: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<Memory>> {
    store
        .get_by_topic(topic, Some(limit), Some(offset))
        .context("Failed to load memories for topic with offset")
}

fn search_memories(store: &SqliteStore, query: &str, limit: usize) -> Result<Vec<Memory>> {
    store
        .search_by_keywords(query, limit)
        .context("Failed to search memories")
}

fn handle_tab_switch(app: &mut AppState, store: &SqliteStore, prev_tab: Tab) -> Result<()> {
    // Only load data when switching TO Memories tab
    if app.current_tab == Tab::Memories && prev_tab != Tab::Memories {
        // Load topics
        app.topics_list = load_topics(store)?;
        if !app.topics_list.is_empty() {
            app.selected_topic_index = Some(0);
            // Load memories for first topic
            app.memories_list = load_memories_for_topic(store, &app.topics_list[0].0, Some(50))?;
            if !app.memories_list.is_empty() {
                app.selected_memory_index = Some(0);
                app.memories_list_state.select(Some(0));
            }
        }
        app.pagination_offset = 0;
    }
    Ok(())
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
        Tab::Memories => render_memories_tab(f, app, chunks[1]),
        Tab::Backup => render_backup_tab(f, app, chunks[1]),
        Tab::Help => render_help_tab(f, app, chunks[1]),
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

            if app.current_tab == Tab::Memories {
                if app.show_delete_confirmation {
                    parts = vec![Span::styled(
                        "Delete memory? (y/n or Esc to cancel)",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )];
                } else {
                    parts.push(Span::raw(
                        " | /:Search | f:Filter | e:Export | d:Delete | PgUp/PgDn",
                    ));
                }
            }

            if let Some(ref user) = app.filter_user {
                parts.push(Span::raw(format!(" | Filter: {}", user)));
            }

            if let Some(ref query) = app.search_query {
                parts.push(Span::raw(format!(" | Search: {}", query)));
            }

            if let Some(ref importance) = app.memory_filter_importance {
                parts.push(Span::raw(format!(" | Importance: {}", importance)));
            }

            if let Some(ref query) = app.memory_search_query {
                parts.push(Span::raw(format!(" | Memory Search: {}", query)));
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
        InputMode::MemorySearch => vec![
            Span::styled("Memory Search: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled(
                " (Enter to apply, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        InputMode::ExportPath => vec![
            Span::styled("Export Path: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.input_buffer),
            Span::styled(
                " (Enter to export, Esc to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ],
        InputMode::ImportPath => vec![
            Span::styled("Import Path: ", Style::default().fg(Color::Yellow)),
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
        Line::from("  1/2/3/4/5/6 - Jump to tab"),
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
        Line::from("  e - Export (Memories/Backup tabs)"),
        Line::from("  d - Delete (Memories tab)"),
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

// ========== Memories Tab ==========

fn render_memories_tab(f: &mut Frame, app: &AppState, area: Rect) {
    // Split into topic list (left 40%) and memory detail (right 60%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Render topics list
    let items: Vec<ListItem> = app
        .topics_list
        .iter()
        .map(|(topic, count)| {
            let line = Line::from(vec![
                Span::styled(topic, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("({})", count), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    if let Some(idx) = app.selected_topic_index {
        list_state.select(Some(idx));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Topics ({})", app.topics_list.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // Render memory detail panel
    if let Some(topic) = app.selected_topic() {
        render_topic_detail(f, app, topic, chunks[1]);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Memory Detail");
        let text = Paragraph::new("Select a topic to view memories").block(block);
        f.render_widget(text, chunks[1]);
    }
}

fn render_topic_detail(f: &mut Frame, app: &AppState, topic: &str, area: Rect) {
    // Filter memories by selected topic
    let topic_memories: Vec<&Memory> = app
        .memories_list
        .iter()
        .filter(|m| m.topic == topic)
        .collect();

    if topic_memories.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Topic: {}", topic));
        let text = Paragraph::new("No memories in this topic").block(block);
        f.render_widget(text, area);
        return;
    }

    // Show first memory detail (in Phase 2, we'll add selection within topic)
    let memory = topic_memories[0];

    let detail_text = vec![
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::Yellow)),
            Span::raw(&memory.id),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Summary: ",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(Span::raw(&memory.summary)),
        Line::from(""),
        Line::from(vec![
            Span::styled("Importance: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                memory.importance.to_string(),
                Style::default().fg(match memory.importance {
                    alejandria_storage::Importance::Critical => Color::Red,
                    alejandria_storage::Importance::High => Color::Magenta,
                    alejandria_storage::Importance::Medium => Color::Yellow,
                    alejandria_storage::Importance::Low => Color::Gray,
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Weight: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.2}", memory.weight)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Access Count: ", Style::default().fg(Color::Yellow)),
            Span::raw(memory.access_count.to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Created: ", Style::default().fg(Color::Yellow)),
            Span::raw(memory.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]),
        Line::from(vec![
            Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
            Span::raw(memory.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]),
    ];

    let paragraph = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Topic: {} ({} memories)",
            topic,
            topic_memories.len()
        )))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

// ========== Backup Tab ==========

fn render_backup_tab(f: &mut Frame, _app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Backup & Restore (Coming Soon)");
    let text = Paragraph::new("Backup tab - Phase 3 implementation").block(block);
    f.render_widget(text, area);
}

// ========== Help Tab ==========

fn render_help_tab(f: &mut Frame, app: &AppState, area: Rect) {
    let help_lines = vec![
        Line::from(Span::styled(
            "Alejandría TUI - Interactive Admin Dashboard",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "NAVIGATION",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  Tab / Shift+Tab       - Switch between tabs"),
        Line::from("  1/2/3/4/5/6           - Jump directly to tab"),
        Line::from("  j / ↓                 - Move down in list"),
        Line::from("  k / ↑                 - Move up in list"),
        Line::from("  gg                    - Go to first item"),
        Line::from("  G                     - Go to last item"),
        Line::from("  q / Ctrl+C            - Quit application"),
        Line::from(""),
        Line::from(Span::styled(
            "API KEYS TAB",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  r                     - Revoke selected key"),
        Line::from("  R                     - Revoke all keys for user"),
        Line::from("  f                     - Filter by username"),
        Line::from("  /                     - Search (username/desc/id)"),
        Line::from("  c                     - Clear filters"),
        Line::from("  t                     - Toggle showing revoked keys"),
        Line::from(""),
        Line::from(Span::styled(
            "MEMORIES TAB",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  /                     - Search memories (FTS5)"),
        Line::from("  f                     - Filter by importance"),
        Line::from("  e                     - Export selected memory"),
        Line::from("  d                     - Delete with confirmation"),
        Line::from("  Enter                 - View memory detail"),
        Line::from(""),
        Line::from(Span::styled(
            "BACKUP TAB",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  e                     - Start export wizard"),
        Line::from("  i                     - Start import wizard"),
        Line::from("  Format options: JSON, CSV, Markdown"),
        Line::from(""),
        Line::from(Span::styled(
            "HELP TAB",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  j/k or ↑/↓            - Scroll help content"),
        Line::from(""),
        Line::from(Span::styled(
            "DATABASE STATS",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  View Stats tab for:"),
        Line::from("    - API key usage by user"),
        Line::from("    - Memory count by topic"),
        Line::from("    - Temporal decay statistics"),
        Line::from("    - Storage usage metrics"),
    ];

    let paragraph = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help & Documentation"),
        )
        .scroll((app.help_scroll_offset as u16, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
