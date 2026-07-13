//! OpenSunstar TUI Dashboard — 全屏治理仪表盘
//!
//! 编译条件: `cfg(feature = "tui")`
//! 入口: `run_dashboard(state)` — 由 main.rs 在无子命令 + TTY 时调用

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

use open_sunstar_lib::{cli_api, AppState};

// ── Terminal Cleanup Guard ───────────────────────────────────

/// RAII guard that restores terminal state on drop (including panic).
/// Prevents the terminal from being stuck in raw mode / alternate screen
/// when the main loop encounters an error.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

// ── App State ────────────────────────────────────────────────

struct App {
    project_contexts: Vec<ProjectSummary>,
    selected_project: usize,
    selected_action: usize,
    active_panel: Panel,
    status_msg: String,
    should_quit: bool,
    table_state: TableState,
}

#[derive(Clone)]
struct ProjectSummary {
    name: String,
    path: String,
    stage: String,
    workspace_exists: bool,
    has_flow_profile: bool,
    has_design_contract: bool,
    recipe_count: usize,
    specs_exists: bool,
    total_assets: u32,
    has_flow_config: bool,
    contract_count: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum Panel {
    Projects,
    Actions,
}

const QUICK_ACTIONS: &[(&str, &str)] = &[
    ("Drift Check", "os drift check"),
    ("Readiness Score", "os readiness score"),
    ("Project Status", "os project status"),
    ("Flow Validate", "os flow validate --strict"),
    ("Doctor", "os doctor"),
];

// ── Data Loading ─────────────────────────────────────────────

fn load_dashboard_data(state: &AppState) -> Vec<ProjectSummary> {
    let projects = cli_api::cli_project_list(state).unwrap_or_default();
    projects
        .iter()
        .map(|p| {
            let ctx = cli_api::cli_project_context(state, &p.path).ok();
            let asset_total = ctx
                .as_ref()
                .map(|c| {
                    let ac = &c.asset_counts;
                    ac.mcp
                        + ac.skills
                        + ac.prompts
                        + ac.commands
                        + ac.hooks
                        + ac.ignore
                        + ac.permissions
                        + ac.subagents
                })
                .unwrap_or(0);
            ProjectSummary {
                name: p.name.clone(),
                path: p.path.clone(),
                stage: p.stage.clone(),
                workspace_exists: ctx.as_ref().map(|c| c.workspace_exists).unwrap_or(false),
                has_flow_profile: ctx.as_ref().map(|c| c.has_flow_profile).unwrap_or(false),
                has_design_contract: ctx.as_ref().map(|c| c.has_design_contract).unwrap_or(false),
                recipe_count: ctx.as_ref().map(|c| c.recipe_count).unwrap_or(0),
                specs_exists: ctx.as_ref().map(|c| c.specs_exists).unwrap_or(false),
                total_assets: asset_total,
                has_flow_config: ctx.as_ref().map(|c| c.has_flow_config).unwrap_or(false),
                contract_count: ctx.as_ref().map(|c| c.contract_count).unwrap_or(0),
            }
        })
        .collect()
}

// ── Main Entry ───────────────────────────────────────────────

pub fn run_dashboard(state: &AppState) -> Result<(), String> {
    // Terminal setup
    enable_raw_mode().map_err(|e| e.to_string())?;
    io::stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| e.to_string())?;

    // Guard ensures cleanup even on panic or early return
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    terminal.clear().map_err(|e| e.to_string())?;

    // Load data
    let project_contexts = load_dashboard_data(state);

    let mut app = App {
        project_contexts,
        selected_project: 0,
        selected_action: 0,
        active_panel: Panel::Projects,
        status_msg: "Ready".to_string(),
        should_quit: false,
        table_state: {
            let mut ts = TableState::default();
            ts.select(Some(0));
            ts
        },
    };

    // Main loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal
            .draw(|f| render(f, &app))
            .map_err(|e| e.to_string())?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key(&mut app, key.code, key.modifiers, state);
                    }
                }
                Event::Resize(_, _) => {
                    // ratatui handles resize automatically on next draw
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
        if app.should_quit {
            break;
        }
    }

    // Guard handles cleanup automatically
    Ok(())
}

// ── Event Handling ───────────────────────────────────────────

fn handle_key(app: &mut App, key: KeyCode, modifiers: KeyModifiers, state: &AppState) {
    // Ctrl+C always quits
    if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                Panel::Projects => Panel::Actions,
                Panel::Actions => Panel::Projects,
            };
        }
        KeyCode::Up => match app.active_panel {
            Panel::Projects => {
                if app.selected_project > 0 {
                    app.selected_project -= 1;
                    app.table_state.select(Some(app.selected_project));
                    update_detail_status(app);
                }
            }
            Panel::Actions => {
                if app.selected_action > 0 {
                    app.selected_action -= 1;
                }
            }
        },
        KeyCode::Down => match app.active_panel {
            Panel::Projects => {
                if !app.project_contexts.is_empty()
                    && app.selected_project < app.project_contexts.len() - 1
                {
                    app.selected_project += 1;
                    app.table_state.select(Some(app.selected_project));
                    update_detail_status(app);
                }
            }
            Panel::Actions => {
                if app.selected_action < QUICK_ACTIONS.len() - 1 {
                    app.selected_action += 1;
                }
            }
        },
        KeyCode::PageUp => {
            if app.active_panel == Panel::Projects && app.selected_project >= 5 {
                app.selected_project -= 5;
            } else if app.active_panel == Panel::Projects {
                app.selected_project = 0;
            }
            app.table_state.select(Some(app.selected_project));
            update_detail_status(app);
        }
        KeyCode::PageDown => {
            if app.active_panel == Panel::Projects {
                let max = app.project_contexts.len().saturating_sub(1);
                app.selected_project = (app.selected_project + 5).min(max);
                app.table_state.select(Some(app.selected_project));
                update_detail_status(app);
            }
        }
        KeyCode::Home => {
            if app.active_panel == Panel::Projects && !app.project_contexts.is_empty() {
                app.selected_project = 0;
                app.table_state.select(Some(0));
                update_detail_status(app);
            }
        }
        KeyCode::End => {
            if app.active_panel == Panel::Projects && !app.project_contexts.is_empty() {
                app.selected_project = app.project_contexts.len() - 1;
                app.table_state.select(Some(app.selected_project));
                update_detail_status(app);
            }
        }
        KeyCode::Enter => match app.active_panel {
            Panel::Projects => {
                update_detail_status(app);
            }
            Panel::Actions => {
                let (_, cmd) = QUICK_ACTIONS[app.selected_action];
                // Strip the "os " prefix to show just the subcommand portion
                let display_cmd = cmd.strip_prefix("os ").unwrap_or(cmd);
                app.status_msg =
                    format!("Hint: exit TUI (q) and run `os {display_cmd}` to execute");
            }
        },
        KeyCode::Char('r') => {
            app.status_msg = "Refreshing...".to_string();
            let new_contexts = load_dashboard_data(state);
            app.project_contexts = new_contexts;
            // Clamp selection if list shrank
            if !app.project_contexts.is_empty() {
                app.selected_project = app.selected_project.min(app.project_contexts.len() - 1);
            } else {
                app.selected_project = 0;
            }
            app.table_state.select(Some(app.selected_project));
            app.status_msg = format!("Refreshed — {} projects loaded", app.project_contexts.len());
        }
        _ => {}
    }
}

fn update_detail_status(app: &mut App) {
    if let Some(ps) = app.project_contexts.get(app.selected_project) {
        app.status_msg = format!(
            "{} — ws:{} flow:{} design:{} specs:{} assets:{}",
            ps.name,
            yn(ps.workspace_exists),
            yn(ps.has_flow_profile),
            yn(ps.has_design_contract),
            yn(ps.specs_exists),
            ps.total_assets,
        );
    }
}

fn yn(b: bool) -> &'static str {
    if b {
        "✓"
    } else {
        "·"
    }
}

// ── Rendering ────────────────────────────────────────────────

fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Summary cards
            Constraint::Min(8),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_summary_cards(f, chunks[1], app);
    render_main_content(f, chunks[2], app);
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " OpenSunstar Dashboard  |  {} projects  |  {} ",
        app.project_contexts.len(),
        app.status_msg,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);
}

fn render_summary_cards(f: &mut Frame, area: Rect, app: &App) {
    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Card 1: Orchestration
    let ws_count = app
        .project_contexts
        .iter()
        .filter(|p| p.workspace_exists)
        .count();
    let card1 = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{ws_count}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("Workspaces")),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Orchestration "),
    )
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(card1, cards[0]);

    // Card 2: Flow Profiles
    let fp_count = app
        .project_contexts
        .iter()
        .filter(|p| p.has_flow_profile)
        .count();
    let card2 = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{fp_count}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("Flow Profiles")),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Workflow "))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(card2, cards[1]);

    // Card 3: Design Contracts
    let dc_count = app
        .project_contexts
        .iter()
        .filter(|p| p.has_design_contract)
        .count();
    let card3 = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{dc_count}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("Design Contracts")),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Design "))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(card3, cards[2]);

    // Card 4: Total Assets
    let total_assets: u32 = app.project_contexts.iter().map(|p| p.total_assets).sum();
    let card4 = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("{total_assets}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("Total Assets")),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Assets "))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(card4, cards[3]);
}

fn render_main_content(f: &mut Frame, area: Rect, app: &App) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    render_projects_table(f, panels[0], app);
    render_right_panel(f, panels[1], app);
}

fn render_projects_table(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Projects;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let header_cells = ["Name", "Stage", "WS", "Flow", "Design", "Specs", "Assets"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .project_contexts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == app.selected_project;
            let style = if selected && is_active {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if selected && !is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            let status_style = |ok: bool| {
                if ok {
                    Span::styled("✓", Style::default().fg(Color::Green))
                } else {
                    Span::styled("·", Style::default().fg(Color::DarkGray))
                }
            };
            Row::new(vec![
                Cell::from(p.name.clone()),
                Cell::from(p.stage.clone()),
                Cell::from(Line::from(status_style(p.workspace_exists))),
                Cell::from(Line::from(status_style(p.has_flow_profile))),
                Cell::from(Line::from(status_style(p.has_design_contract))),
                Cell::from(Line::from(status_style(p.specs_exists))),
                Cell::from(format!("{}", p.total_assets)),
            ])
            .style(style)
        })
        .collect();

    // Column widths sum to ~100% for optimal screen usage
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30), // Name
            Constraint::Percentage(12), // Stage
            Constraint::Percentage(7),  // WS
            Constraint::Percentage(7),  // Flow
            Constraint::Percentage(10), // Design
            Constraint::Percentage(8),  // Specs
            Constraint::Percentage(10), // Assets
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(" Projects ({}) ", app.project_contexts.len()),
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )),
    );

    // Clone TableState for rendering (TableState requires &mut for render_with_state)
    let mut ts = app.table_state.clone();
    f.render_stateful_widget(table, area, &mut ts);
}

fn render_right_panel(f: &mut Frame, area: Rect, app: &App) {
    let panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_project_detail(f, panels[0], app);
    render_quick_actions(f, panels[1], app);
}

fn render_project_detail(f: &mut Frame, area: Rect, app: &App) {
    let content = if let Some(ps) = app.project_contexts.get(app.selected_project) {
        vec![
            Line::from(vec![
                Span::styled("Path:      ", Style::default().fg(Color::DarkGray)),
                Span::raw(ps.path.clone()),
            ]),
            Line::from(vec![
                Span::styled("Stage:     ", Style::default().fg(Color::DarkGray)),
                Span::raw(ps.stage.clone()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Recipes:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", ps.recipe_count),
                    Style::default().fg(if ps.recipe_count > 0 {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Contracts: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", ps.contract_count),
                    Style::default().fg(if ps.contract_count > 0 {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Assets:    ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", ps.total_assets),
                    Style::default().fg(if ps.total_assets > 0 {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("FlowCfg:   ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    yn(ps.has_flow_config),
                    Style::default().fg(if ps.has_flow_config {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No projects registered",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let detail = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Project Detail "),
    );
    f.render_widget(detail, area);
}

fn render_quick_actions(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Actions;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = QUICK_ACTIONS
        .iter()
        .enumerate()
        .map(|(i, (label, cmd))| {
            let style = if i == app.selected_action && is_active {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {label}"), style),
                Span::styled(format!("  [{cmd}]"), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                " Quick Actions ",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " ↑↓",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" navigate  "),
        Span::styled(
            "PgUp/Dn",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll  "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" switch  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" select  "),
        Span::styled(
            "r",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" refresh  "),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" quit"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(footer, area);
}
