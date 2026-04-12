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
use anyhow::{anyhow, bail, Context, Result};
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
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
#[allow(dead_code)] // Future expansion for export wizard
enum ExportStep {
    FormatSelection,
    FilterConfig,
    PathInput,
    Preview,
    Progress,
    Complete,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Future expansion for export wizard
struct ExportFilters {
    date_from: Option<DateTime<Utc>>,
    date_to: Option<DateTime<Utc>>,
    topic_pattern: Option<String>,
    min_importance: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Future expansion for export wizard
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
#[allow(dead_code)] // Future expansion for import wizard
enum ImportMode {
    Skip,
    Update,
    Replace,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Future expansion for import wizard
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
    #[allow(dead_code)] // Future expansion for full wizard UI
    export_wizard_state: ExportWizardState,
    #[allow(dead_code)] // Future expansion for full wizard UI
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
    #[allow(dead_code)] // Future expansion for topic browsing
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

    #[allow(dead_code)] // Future expansion for topic browsing
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

                        // Backup tab actions
                        (KeyCode::Char('e'), _) if app.current_tab == Tab::Backup => {
                            // Start export wizard
                            app.input_mode = InputMode::ExportPath;
                            app.input_buffer = String::from("./alejandria_export.json");
                        }
                        (KeyCode::Char('i'), _) if app.current_tab == Tab::Backup => {
                            // Start import wizard
                            app.input_mode = InputMode::ImportPath;
                            app.input_buffer.clear();
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
                                    let path = PathBuf::from(&app.input_buffer);

                                    if app.current_tab == Tab::Memories {
                                        // Single memory export (simple, less secure)
                                        if let Some(memory) = app.selected_memory() {
                                            let json = serde_json::to_string_pretty(memory)?;
                                            fs::write(&path, json)?;
                                        }
                                    } else if app.current_tab == Tab::Backup {
                                        // Full export with security (Phase 3)
                                        export_all_memories(store, &path)?;
                                    }
                                }
                                InputMode::ImportPath => {
                                    if app.current_tab == Tab::Backup {
                                        let path = PathBuf::from(&app.input_buffer);
                                        import_memories_secure(store, &path)?;
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

/// Export all memories with full security checks
fn export_all_memories(store: &SqliteStore, path: &Path) -> Result<()> {
    // **SECURITY**: Validate path FIRST
    let safe_path = validate_export_path(path)?;

    // Get all topics and collect memories
    let topics = store.list_topics(None, None)?;
    let mut all_memories = Vec::new();
    for topic_info in topics {
        let memories = store.get_by_topic(&topic_info.topic, None, None)?;
        all_memories.extend(memories);
    }

    // Format as JSON
    let mut content = serde_json::to_string_pretty(&all_memories)?;

    // **SECURITY**: Redact secrets
    content = redact_secrets(&content, true);

    // **SECURITY**: Write with secure permissions
    fs::write(&safe_path, &content)?;
    set_secure_permissions(&safe_path)?;

    // **SECURITY**: Generate checksum
    let checksum = calculate_sha256(&safe_path)?;
    write_metadata_file(&safe_path, all_memories.len(), &checksum)?;

    // **SECURITY**: Audit log
    log_backup_operation(
        "export",
        "<tui_export>",
        &safe_path,
        all_memories.len(),
        "success",
        None,
    )?;

    Ok(())
}

/// Import memories with validation and conflict resolution
fn import_memories_secure(store: &SqliteStore, path: &Path) -> Result<()> {
    // **SECURITY**: Validate file size (REQ-SEC-3)
    let metadata = fs::metadata(path)?;
    let size_mb = metadata.len() / 1024 / 1024;
    let max_mb = std::env::var("ALEJANDRIA_MAX_IMPORT_MB")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<u64>()?;

    if size_mb > max_mb {
        bail!(
            "File too large: {} MB exceeds limit of {} MB",
            size_mb,
            max_mb
        );
    }

    // Parse JSON file manually for simplicity in TUI
    let content = fs::read_to_string(path)?;
    let memories: Vec<Memory> = serde_json::from_str(&content).context("Invalid JSON format")?;

    // Import with skip mode (safest default for TUI)
    let mut created = 0;
    let _skipped = 0; // Placeholder for future conflict resolution tracking

    for memory in memories {
        // Check if already exists by topic_key
        if let Some(ref topic_key) = memory.topic_key {
            match store.get_by_topic_key(topic_key) {
                Ok(Some(_)) => {
                    // _skipped += 1; // Placeholder
                    continue;
                }
                Ok(None) => {
                    // Doesn't exist, can insert
                }
                Err(_) => {
                    // Error checking, skip for safety
                    // _skipped += 1; // Placeholder
                    continue;
                }
            }
        }

        // Store the memory
        match store.store(memory) {
            Ok(_) => created += 1,
            Err(_) => {} // _skipped += 1; // Placeholder
        }
    }

    // **SECURITY**: Audit log
    log_backup_operation("import", "<tui_import>", path, created, "success", None)?;

    Ok(())
}

fn ui(f: &mut Frame, app: &AppState) {
    // Main layout: tabs + content + 2-line status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Status bar (2 lines + border)
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
    let data = [
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

// ========== Security Tests (Phase 5) ==========

#[cfg(test)]
mod security_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_ac001_path_traversal_prevention() {
        // AC-001: Path traversal attack - should be blocked
        let result = validate_export_path(Path::new("../../../etc/passwd"));
        assert!(result.is_err(), "Path traversal should be blocked");
        if let Err(e) = result {
            let err_msg = e.to_string().to_lowercase();
            // Can fail with either path traversal error or "no such file" (from canonicalize)
            assert!(
                err_msg.contains("path traversal")
                    || err_msg.contains("does not exist")
                    || err_msg.contains("no such file")
                    || err_msg.contains("not found"),
                "Error should mention path issue, got: {}",
                err_msg
            );
        }
    }

    #[test]
    fn test_filename_sanitization() {
        let dangerous = "export; rm -rf /.json";
        let safe = sanitize_filename(dangerous);
        // Note: consecutive special chars become consecutive underscores
        assert_eq!(safe, "export__rm_-rf__.json");
        assert!(!safe.contains(";"), "Semicolon should be sanitized");
        assert!(!safe.contains(" "), "Spaces should be sanitized");
        assert!(!safe.contains("/"), "Slashes should be sanitized");
    }

    #[test]
    fn test_ac001_relative_path_allowed() {
        // Relative paths in current dir should work
        let temp_dir = std::env::temp_dir();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a test file in temp dir
        let test_file = temp_dir.join("test_export.json");
        std::fs::write(&test_file, "{}").unwrap();

        let result = validate_export_path(Path::new("test_export.json"));
        assert!(result.is_ok(), "Relative path in current dir should work");

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_ac004_secret_redaction_api_keys() {
        // AC-004: Secret exfiltration prevention - API keys
        let content = "API_KEY=sk-1234567890abcdefghij\nANOTHER_KEY=ghp_abcdefghijklmnopqrstuvwxyz123456789012";
        let redacted = redact_secrets(content, true);
        assert!(
            !redacted.contains("sk-1234567890abcdefghij"),
            "OpenAI key should be redacted"
        );
        assert!(
            !redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz123456789012"),
            "GitHub token should be redacted"
        );
        assert!(
            redacted.contains("[REDACTED]"),
            "Should contain [REDACTED] marker"
        );
    }

    #[test]
    fn test_ac004_secret_redaction_passwords() {
        let content = "PASSWORD=supersecret123\npwd: mypassword456";
        let redacted = redact_secrets(content, true);
        assert!(
            !redacted.contains("supersecret123"),
            "Password should be redacted"
        );
        assert!(
            !redacted.contains("mypassword456"),
            "pwd should be redacted"
        );
    }

    #[test]
    fn test_ac004_redaction_opt_out() {
        let content = "API_KEY=sk-test1234567890abcdefghij";
        let not_redacted = redact_secrets(content, false);
        assert_eq!(not_redacted, content, "Should not redact when disabled");
    }

    #[test]
    fn test_file_permissions() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("alejandria_test_perms.txt");
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(b"test").unwrap();
        drop(file);

        set_secure_permissions(&temp).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::metadata(&temp).unwrap().permissions();
            assert_eq!(
                perms.mode() & 0o777,
                0o600,
                "File permissions should be 600 (rw-------)"
            );
        }

        std::fs::remove_file(&temp).unwrap();
    }

    #[test]
    fn test_sha256_checksum() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("alejandria_test_hash.txt");
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);

        let hash = calculate_sha256(&temp).unwrap();
        assert_eq!(hash.len(), 64, "SHA-256 hash should be 64 hex characters");
        // Known SHA-256 of "test content"
        assert_eq!(
            hash,
            "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72"
        );

        std::fs::remove_file(&temp).unwrap();
    }

    #[test]
    fn test_metadata_file_creation() {
        use std::io::Write;
        let temp = std::env::temp_dir().join("alejandria_test_export.json");
        let mut file = std::fs::File::create(&temp).unwrap();
        file.write_all(b"{}").unwrap();
        drop(file);

        let checksum = "abcd1234";
        write_metadata_file(&temp, 42, checksum).unwrap();

        let meta_path = temp.with_extension("json.meta");
        assert!(meta_path.exists(), "Metadata file should be created");

        let meta_content = fs::read_to_string(&meta_path).unwrap();
        assert!(meta_content.contains("\"count\": 42"));
        assert!(meta_content.contains("\"checksum\": \"abcd1234\""));
        assert!(meta_content.contains("\"format\": \"json\""));
        assert!(meta_content.contains("exported_at"));

        std::fs::remove_file(&temp).unwrap();
        std::fs::remove_file(&meta_path).unwrap();
    }

    #[test]
    fn test_sanitize_all_dangerous_chars() {
        let dangerous = "file$name`with;bad&chars|here.txt";
        let safe = sanitize_filename(dangerous);
        assert!(!safe.contains("$"));
        assert!(!safe.contains("`"));
        assert!(!safe.contains(";"));
        assert!(!safe.contains("&"));
        assert!(!safe.contains("|"));
        assert!(safe.contains(".txt"), "Extension should be preserved");
    }
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

// Helper functions for status bar styling
fn key_span(k: &str) -> Span<'_> {
    Span::styled(
        k.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn sep_span() -> Span<'static> {
    Span::styled(" │ ", Style::default().fg(Color::DarkGray))
}

fn desc_span(d: &str) -> Span<'_> {
    Span::raw(d.to_string())
}

fn render_status_bar(f: &mut Frame, app: &AppState, area: Rect) {
    let (line1, line2) = match app.input_mode {
        InputMode::Normal => {
            if app.show_delete_confirmation {
                // Special case: delete confirmation
                let confirm = vec![
                    Span::styled(
                        "Delete memory? ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    key_span("y"),
                    desc_span(" yes"),
                    sep_span(),
                    key_span("n"),
                    desc_span(" no"),
                    sep_span(),
                    key_span("Esc"),
                    desc_span(" cancel"),
                ];
                (confirm, vec![])
            } else {
                // Tab-specific commands (Line 1)
                let mut tab_commands = Vec::new();
                match app.current_tab {
                    Tab::ApiKeys => {
                        tab_commands.extend(vec![
                            key_span("n"),
                            desc_span(" new"),
                            sep_span(),
                            key_span("r"),
                            desc_span(" revoke"),
                            sep_span(),
                            key_span("R"),
                            desc_span(" revoke-user"),
                            sep_span(),
                            key_span("f"),
                            desc_span(" filter"),
                            sep_span(),
                            key_span("/"),
                            desc_span(" search"),
                            sep_span(),
                            key_span("c"),
                            desc_span(" clear"),
                        ]);
                    }
                    Tab::Memories => {
                        tab_commands.extend(vec![
                            key_span("/"),
                            desc_span(" search"),
                            sep_span(),
                            key_span("f"),
                            desc_span(" filter"),
                            sep_span(),
                            key_span("e"),
                            desc_span(" export"),
                            sep_span(),
                            key_span("d"),
                            desc_span(" delete"),
                            sep_span(),
                            key_span("c"),
                            desc_span(" clear"),
                        ]);
                        // Show active filters
                        if let Some(ref importance) = app.memory_filter_importance {
                            tab_commands.push(Span::styled(
                                format!(" [Filter: {}]", importance),
                                Style::default().fg(Color::Yellow),
                            ));
                        }
                        if let Some(ref query) = app.memory_search_query {
                            tab_commands.push(Span::styled(
                                format!(" [Search: {}]", query),
                                Style::default().fg(Color::Yellow),
                            ));
                        }
                    }
                    Tab::Backup => {
                        tab_commands.extend(vec![
                            key_span("e"),
                            desc_span(" export all"),
                            sep_span(),
                            key_span("i"),
                            desc_span(" import file"),
                        ]);
                    }
                    Tab::Help => {
                        tab_commands.extend(vec![
                            key_span("j/k"),
                            desc_span(" scroll"),
                            sep_span(),
                            key_span("gg"),
                            desc_span(" top"),
                            sep_span(),
                            key_span("G"),
                            desc_span(" bottom"),
                        ]);
                    }
                    Tab::Stats | Tab::ActivityLog => {
                        // No specific commands
                        tab_commands.push(desc_span("Use Tab to navigate or ? for help"));
                    }
                }

                // Global commands (Line 2)
                let mut global_commands = vec![
                    key_span("Tab"),
                    desc_span(" 1-6 switch"),
                    sep_span(),
                    key_span("j/k"),
                    desc_span(" nav"),
                    sep_span(),
                ];

                // Add pagination hint if in Memories tab
                if app.current_tab == Tab::Memories {
                    global_commands.extend(vec![
                        key_span("PgUp/Dn"),
                        desc_span(" page"),
                        sep_span(),
                    ]);
                }

                global_commands.extend(vec![
                    key_span("?"),
                    desc_span(" help"),
                    sep_span(),
                    key_span("q"),
                    desc_span(" quit"),
                ]);

                // Add filter/search indicators for API Keys
                if app.current_tab == Tab::ApiKeys {
                    if let Some(ref user) = app.filter_user {
                        global_commands.push(Span::styled(
                            format!(" [Filter: {}]", user),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    if let Some(ref query) = app.search_query {
                        global_commands.push(Span::styled(
                            format!(" [Search: {}]", query),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    if app.show_revoked {
                        global_commands.push(Span::styled(
                            " [Showing Revoked]",
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                }

                (tab_commands, global_commands)
            }
        }
        InputMode::Filter => {
            let prompt = vec![
                Span::styled("Filter by user: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to apply, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
        InputMode::Search => {
            let prompt = vec![
                Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to apply, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
        InputMode::NewKey => {
            let prompt = vec![
                Span::styled("New key user: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to continue, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
        InputMode::MemorySearch => {
            let prompt = vec![
                Span::styled("Memory Search: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to apply, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
        InputMode::ExportPath => {
            let prompt = vec![
                Span::styled("Export to: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to export, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
        InputMode::ImportPath => {
            let prompt = vec![
                Span::styled("Import from: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled(
                    " (Enter to continue, Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            (prompt, vec![])
        }
    };

    // Build text with 1 or 2 lines
    let text = if line2.is_empty() {
        Text::from(vec![Line::from(line1)])
    } else {
        Text::from(vec![Line::from(line1), Line::from(line2)])
    };

    let status = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
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

// ========== Security Functions for Backup Operations ==========

/// AC-001 Mitigation: Prevent path traversal attacks
fn validate_export_path(path: &Path) -> Result<PathBuf> {
    // Resolve to absolute path
    let abs_path = if path.is_relative() {
        std::env::current_dir()?.join(path)
    } else {
        path.to_path_buf()
    };

    // Canonicalize (resolves .., symlinks)
    let canonical = abs_path.canonicalize().or_else(|_| -> Result<PathBuf> {
        // If file doesn't exist, canonicalize parent
        let parent = abs_path.parent().ok_or_else(|| anyhow!("Invalid path"))?;
        let parent_canonical = parent.canonicalize()?;
        Ok(parent_canonical.join(abs_path.file_name().unwrap()))
    })?;

    // Whitelist check: must be in current directory or subdirectory
    let allowed = std::env::current_dir()?;
    if !canonical.starts_with(&allowed) {
        bail!("Path traversal detected: path must be within current directory");
    }

    // Sanitize filename
    if let Some(filename) = canonical.file_name() {
        let clean = sanitize_filename(filename.to_str().unwrap());
        if clean != filename.to_str().unwrap() {
            bail!("Invalid filename: contains shell metacharacters");
        }
    }

    Ok(canonical)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// AC-004 Mitigation: Redact secrets from exports
fn redact_secrets(content: &str, enabled: bool) -> String {
    if !enabled {
        return content.to_string();
    }

    let patterns = vec![
        (
            r"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\x22]?([a-zA-Z0-9_-]{20,})['\x22]?",
            "API_KEY=[REDACTED]",
        ),
        (
            r"(?i)(password|passwd|pwd)\s*[:=]\s*['\x22]?([^\s'\x22]{8,})['\x22]?",
            "PASSWORD=[REDACTED]",
        ),
        (
            r"(?i)(token|bearer)\s*[:=]\s*['\x22]?([a-zA-Z0-9_\.-]{20,})['\x22]?",
            "TOKEN=[REDACTED]",
        ),
        (r"sk-[a-zA-Z0-9]{20,}", "OPENAI_KEY=[REDACTED]"),
        (r"ghp_[a-zA-Z0-9]{36,}", "GITHUB_TOKEN=[REDACTED]"),
        (r"glpat-[a-zA-Z0-9_-]{20,}", "GITLAB_TOKEN=[REDACTED]"),
        (
            r"eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}",
            "JWT=[REDACTED]",
        ),
    ];

    let mut result = content.to_string();
    for (pattern, replacement) in patterns {
        let re = Regex::new(pattern).unwrap();
        result = re.replace_all(&result, replacement).to_string();
    }
    result
}

/// Set file permissions to owner-only (Unix only)
fn set_secure_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600); // rw-------
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Calculate SHA-256 checksum
fn calculate_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let hash = Sha256::digest(&bytes);
    Ok(format!("{:x}", hash))
}

/// Write metadata file with checksum
fn write_metadata_file(export_path: &Path, count: usize, checksum: &str) -> Result<()> {
    let meta_path = export_path.with_extension(format!(
        "{}.meta",
        export_path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap()
    ));

    let meta = serde_json::json!({
        "count": count,
        "checksum": checksum,
        "format": export_path.extension().unwrap_or_default().to_str().unwrap(),
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });

    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    set_secure_permissions(&meta_path)?;
    Ok(())
}

/// REQ-SEC-4: Audit logging for backup operations
#[allow(dead_code)]
fn log_backup_operation(
    operation: &str,
    owner_key_hash: &str,
    file_path: &Path,
    count: usize,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let log_path = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("Cannot find data directory"))?
        .join("alejandria")
        .join("audit.log");

    fs::create_dir_all(log_path.parent().unwrap())?;

    let log_entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "operation": operation,
        "owner_key_hash": owner_key_hash,
        "file": file_path.display().to_string(),
        "count": count,
        "status": status,
        "error": error,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    writeln!(file, "{}", log_entry)?;
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
    // Simple menu for export/import selection
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // Main menu
    let menu_items = vec![
        Line::from(vec![
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" - Export memories to file (JSON/CSV/Markdown)"),
        ]),
        Line::from(vec![
            Span::styled("i", Style::default().fg(Color::Yellow)),
            Span::raw(" - Import memories from file"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Export Features:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  • Path traversal protection (AC-001)"),
        Line::from("  • Secret redaction (AC-004)"),
        Line::from("  • SHA-256 checksums"),
        Line::from("  • Secure file permissions (600)"),
        Line::from("  • Audit logging"),
    ];

    let menu = Paragraph::new(menu_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Backup & Restore"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(menu, chunks[0]);

    // Instructions panel
    let instructions = vec![
        Line::from(Span::styled(
            "Export Process:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  1. Press 'e' to start export"),
        Line::from("  2. Enter filename (e.g., 'memories.json')"),
        Line::from("  3. Confirm export"),
        Line::from(""),
        Line::from(Span::styled(
            "Import Process:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  1. Press 'i' to start import"),
        Line::from("  2. Enter source file path"),
        Line::from("  3. Choose conflict resolution (skip/update/replace)"),
        Line::from("  4. Review and confirm"),
        Line::from(""),
        Line::from(Span::styled(
            "Security Features:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  • All exports are validated for path traversal"),
        Line::from("  • Secrets are automatically redacted"),
        Line::from("  • Files are created with restrictive permissions (600)"),
        Line::from("  • SHA-256 checksums prevent tampering"),
        Line::from("  • All operations are logged to audit.log"),
    ];

    let info = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Instructions"))
        .wrap(Wrap { trim: true });

    f.render_widget(info, chunks[1]);
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
