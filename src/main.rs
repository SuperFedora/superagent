// Copyright 2026 SuperFedora Contributors
// SPDX-License-Identifier: Apache-2.0

mod comms;
mod state;
mod tui;

use crate::comms::{ask_agent, exec_quake_runner, relay_cast, run_daemon, run_watchdog};
use crate::state::AgentState;
use crate::tui::run_tui;
use std::env;
use std::io::{self, IsTerminal};
use std::process::{self, Command};

fn print_help() {
    println!("SuperAgent v0.3.0 — Standalone 16-Bit Autonomous Workstation Agent & Cast Watchdog");
    println!("Usage:");
    println!("  superagent                                  Launch TUI (or print status if non-TTY)");
    println!("  superagent tui | configure                  Launch interactive 16-Bit Agent & Ollama Studio");
    println!("  superagent status                           Show active harness, Ollama VRAM & daemon state");
    println!("  superagent watchdog [--json|--notify]       Run zero-token local telemetry & anomaly watchdog");
    println!("  superagent daemon [--once]                  Run autonomous watchdog & Cast Comms relay");
    println!("  superagent notify <role> <title> [body]     Transmit 16-bit Cast Comms notification (supernotify)");
    println!("  superagent openclaw [status|ask <prompt>]   OpenClaw autonomous harness status or query");
    println!("  superagent ollama [probe|table|chat|ask]    Probe or chat with local/LAN Ollama server");
    println!("  superagent ask <prompt...>                  Query active AI agent or Ollama model");
    println!("  superagent [toggle|show|hide|restart|list-agents|select-agent <id>|next-agent|set-ollama <ip:port>]");
}

fn print_status(st: &AgentState) {
    println!("SuperAgent v0.3.0 — Autonomous Workstation Agent & Cast Watchdog");
    println!("  Active Harness : {} [{}] ({})", st.active_label, st.active_agent, st.active_cmd);
    println!("  OpenClaw State : {}", if st.openclaw_ready { "ready" } else { "standby" });
    println!("  Ollama Server  : http://{} [{}] (Model: {})", st.ollama_host, if st.ollama_online { "ONLINE" } else { "OFFLINE" }, st.ollama_model);
    println!("  Cast Watchdog  : medic, security, navigator, commander, ai (supernotify --cast)");
    println!("  Harnesses      :");
    for a in &st.agents {
        let mk = if a.active { "[*]" } else { "   " };
        let inst = if a.installed { "[OK]" } else { "[--]" };
        println!("    {mk} {:<10} [{:<10}] {:<38} {inst}", a.id, a.category, a.label);
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(first) = args.first().map(|s| s.as_str()) {
        match first {
            "-h" | "--help" | "help" => {
                print_help();
                return;
            }
            "-V" | "--version" | "version" => {
                println!("superagent 0.3.0 (SuperFedora/superagent — Rust 1.99 Native Engine)");
                return;
            }
            "tui" | "configure" => {
                process::exit(run_tui());
            }
            _ => {}
        }
    }

    let mut st = AgentState::load();
    if args.is_empty() {
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            process::exit(run_tui());
        } else {
            print_status(&st);
            return;
        }
    }

    let cmd = args[0].as_str();
    match cmd {
        "status" => print_status(&st),
        "watchdog" | "audit" | "health" => {
            let json = args.iter().any(|a| a == "--json");
            let notify = args.iter().any(|a| a == "--notify" || a == "--cast");
            process::exit(run_watchdog(&mut st, json, notify));
        }
        "daemon" => {
            let once = args.iter().any(|a| a == "--once" || a == "-1" || a == "once");
            process::exit(run_daemon(&mut st, once));
        }
        "notify" | "cast" => {
            let role = args.get(1).map(|s| s.as_str()).unwrap_or("ai");
            let title = args.get(2).map(|s| s.as_str()).unwrap_or("SuperAgent Update");
            let body = args.get(3).map(|s| s.as_str()).unwrap_or("Autonomous workstation agent task complete.");
            process::exit(relay_cast(role, title, body));
        }
        "openclaw" | "claw" => {
            let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");
            if sub == "status" {
                println!("OpenClaw Autonomous Harness: {}", if st.openclaw_ready { "READY" } else { "STANDBY" });
                println!("Cast Comms Channel: supernotify --cast ai");
            } else {
                let prompt = args[1..].join(" ");
                let code = ask_agent(&st, &prompt);
                let _ = relay_cast("ai", "OpenClaw Task Completed", &format!("Finished prompt: {prompt}"));
                process::exit(code);
            }
        }
        "ollama" => {
            let sf_ol = st.home.join(".local/bin/sf-ollama");
            let sub = args.get(1).map(|s| s.as_str()).unwrap_or("table");
            let host = args.get(2).map(|s| s.as_str()).unwrap_or(&st.ollama_host);
            let model = args.get(3).map(|s| s.as_str()).unwrap_or(&st.ollama_model);
            let code = Command::new(sf_ol)
                .args([sub, host, model])
                .status()
                .map(|s| s.code().unwrap_or(0))
                .unwrap_or(1);
            process::exit(code);
        }
        "ask" | "query" => {
            let prompt = args[1..].join(" ");
            process::exit(ask_agent(&st, &prompt));
        }
        _ => {
            process::exit(exec_quake_runner(&st, &args));
        }
    }
}
