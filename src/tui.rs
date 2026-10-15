// Copyright 2026 SuperFedora Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::comms::{exec_quake_runner, relay_cast, run_daemon};
use crate::state::AgentState;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn run_tui() -> i32 {
    let mut st = AgentState::load();
    if enable_raw_mode().is_err() {
        return 1;
    }
    let mut stdout = io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen);
    let Ok(mut term) = Terminal::new(CrosstermBackend::new(stdout)) else {
        let _ = disable_raw_mode();
        return 1;
    };

    let mut sel = st.agents.iter().position(|a| a.active).unwrap_or(0);
    let mut msg = format!("SuperAgent v0.1.0 Ready — Active: {} ({})", st.active_label, st.active_agent);

    loop {
        let _ = term.draw(|f| render_ui(f, &st, sel, &msg));

        if event::poll(Duration::from_millis(250)).unwrap_or(false)
            && let Ok(Event::Key(k)) = event::read()
            && k.kind == KeyEventKind::Press
        {
            let total = st.agents.len().max(1);
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down | KeyCode::Char('j') => sel = (sel + 1) % total,
                KeyCode::Up | KeyCode::Char('k') => sel = (sel + total - 1) % total,
                KeyCode::Enter => {
                    if let Some(a) = st.agents.get(sel) {
                        let id = a.id.clone();
                        st.active_agent = id.clone();
                        st.save_env();
                        st.refresh();
                        let _ = Command::new("qs").args(["ipc", "call", "status", "refresh"]).stdout(Stdio::null()).stderr(Stdio::null()).status();
                        msg = format!("Selected active harness: {} ({id})", st.active_label);
                    }
                }
                KeyCode::Char('c') => {
                    let _ = relay_cast(
                        "ai",
                        &format!("SuperAgent // {}", st.active_label),
                        &format!("Autonomous workstation link verified (Ollama: {}).", st.ollama_host),
                    );
                    msg = "Transmitted 16-bit Cast Comms notification via supernotify".into();
                }
                KeyCode::Char('d') => {
                    let _ = run_daemon(&mut st, true);
                    msg = "Executed autonomous watchdog check & Cast Comms relay".into();
                }
                KeyCode::Char('o') => {
                    let _ = disable_raw_mode();
                    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
                    let _ = exec_quake_runner(&st, &["toggle".into()]);
                    let _ = execute!(term.backend_mut(), EnterAlternateScreen);
                    let _ = enable_raw_mode();
                    let _ = term.clear();
                    msg = "Toggled Quake AI drop-down console".into();
                }
                KeyCode::Char('r') => {
                    st.refresh();
                    msg = "Refreshed OpenClaw, coding harnesses & Ollama VRAM models".into();
                }
                _ => {}
            }
        }
    }

    let _ = disable_raw_mode();
    let _ = execute!(term.backend_mut(), LeaveAlternateScreen);
    let _ = term.show_cursor();
    0
}

fn render_ui(f: &mut Frame, st: &AgentState, selected: usize, msg: &str) {
    let pri = st.pri();
    let sec = st.sec();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(12), Constraint::Length(3)])
        .split(f.area());

    let ol_badge = if st.ollama_online { "ONLINE" } else { "OFFLINE" };
    let hdr = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" [::] SUPERAGENT v0.1.0 ", Style::default().fg(Color::Black).bg(pri).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  Active Harness: {} [{}]  |  OpenClaw: {} ", st.active_label, st.active_agent, if st.openclaw_ready { "READY" } else { "STANDBY" }),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Ollama Endpoint: ", Style::default().fg(sec).add_modifier(Modifier::BOLD)),
            Span::raw(format!("http://{} [{}]  |  Pi Model: {}  |  Cast Comms: AGY // OPENCLAW", st.ollama_host, ol_badge, st.ollama_model)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(pri)));
    f.render_widget(hdr, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(rows[1]);

    let items: Vec<ListItem> = st
        .agents
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let is_sel = i == selected;
            let badge = if a.installed { "[READY]" } else { "[--   ]" };
            let b_col = if a.installed { Color::Green } else { Color::DarkGray };
            let act = if a.active { "* " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(if is_sel { " > " } else { "   " }, Style::default().fg(pri).add_modifier(Modifier::BOLD)),
                Span::styled(act, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{badge:<8}"), Style::default().fg(b_col).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:<10} ", a.id),
                    Style::default()
                        .fg(if is_sel { Color::Black } else { Color::White })
                        .bg(if is_sel { pri } else { Color::Reset })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("[{:<10}] ", a.category), Style::default().fg(sec)),
                Span::styled(&a.label, Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(pri))
            .title(Span::styled(" AUTONOMOUS & CODING HARNESSES (Enter=Select Active, o=Quake Console) ", Style::default().fg(pri).add_modifier(Modifier::BOLD))),
    );
    f.render_widget(list, cols[0]);

    let mut ol_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ollama Server : ", Style::default().fg(sec).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("http://{} ({ol_badge})", st.ollama_host),
                Style::default().fg(if st.ollama_online { Color::Green } else { Color::Yellow }).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Active Model  : ", Style::default().fg(sec).add_modifier(Modifier::BOLD)),
            Span::raw(&st.ollama_model),
        ]),
        Line::from(vec![
            Span::styled("  Systemd Unit  : ", Style::default().fg(sec).add_modifier(Modifier::BOLD)),
            Span::raw(if st.daemon_active { "superagent.service (active)" } else { "superagent.service (on-demand)" }),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Detected Ollama Models (/api/ps & /api/tags):", Style::default().fg(pri).add_modifier(Modifier::BOLD))),
    ];
    if st.ollama_models.is_empty() {
        ol_lines.push(Line::from("    [--] No models reported (or Ollama offline)"));
    } else {
        for m in &st.ollama_models {
            let tag = if m.loaded { "[VRAM]" } else { "[DISK]" };
            ol_lines.push(Line::from(format!("    {tag} {:<24} {:<8} [{}] ({})", m.name, m.size, m.quant, m.display)));
        }
    }
    let right_blk = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(sec))
        .title(Span::styled(" OLLAMA VRAM & CAST COMMS DAEMON ", Style::default().fg(sec).add_modifier(Modifier::BOLD)));
    f.render_widget(Paragraph::new(ol_lines).block(right_blk), cols[1]);

    let ftr = Paragraph::new(Line::from(vec![
        Span::styled(" KEYS ", Style::default().fg(Color::Black).bg(sec).add_modifier(Modifier::BOLD)),
        Span::raw(" [Enter] Select Harness  [c] Cast Comms Test  [d] Daemon Check  [o] Quake Console  [r] Refresh  [q] Quit | "),
        Span::styled(msg, Style::default().fg(pri).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(sec)));
    f.render_widget(ftr, rows[2]);
}
