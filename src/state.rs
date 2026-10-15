// Copyright 2026 SuperFedora Contributors
// SPDX-License-Identifier: Apache-2.0

use ratatui::style::Color;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct AgentEntry {
    pub id: String,
    pub category: String,
    pub label: String,
    pub cmd: String,
    pub bin: String,
    pub installed: bool,
    pub active: bool,
}

#[derive(Clone)]
pub struct OllamaModel {
    pub name: String,
    pub loaded: bool,
    pub size: String,
    pub quant: String,
    pub display: String,
}

#[derive(Clone)]
pub struct AgentState {
    pub home: PathBuf,
    pub agent_root: PathBuf,
    pub state_dir: PathBuf,
    pub active_agent: String,
    pub active_label: String,
    pub active_cmd: String,
    pub active_bin: String,
    pub ollama_host: String,
    pub ollama_model: String,
    pub ollama_online: bool,
    pub ollama_models: Vec<OllamaModel>,
    pub agents: Vec<AgentEntry>,
    pub openclaw_ready: bool,
    pub daemon_active: bool,
    pub pri_rgb: (u8, u8, u8),
    pub sec_rgb: (u8, u8, u8),
}

impl AgentState {
    pub fn load() -> Self {
        let home = PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/home/jeryd".into()));
        let state_dir = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"))
            .join("superfedora");
        let _ = fs::create_dir_all(&state_dir);
        let proj = home.join("Projects/superfedora/superagent");
        let agent_root = if proj.join("lib/quake-runner.sh").exists() {
            proj
        } else {
            home.join(".local/share/superagent")
        };
        let mut st = Self {
            home,
            agent_root,
            state_dir,
            active_agent: "agy".into(),
            active_label: "Antigravity CLI (agy)".into(),
            active_cmd: "agy".into(),
            active_bin: "agy".into(),
            ollama_host: "127.0.0.1:11434".into(),
            ollama_model: "llama3.2".into(),
            ollama_online: false,
            ollama_models: Vec::new(),
            agents: Vec::new(),
            openclaw_ready: false,
            daemon_active: false,
            pri_rgb: (122, 162, 247),
            sec_rgb: (187, 154, 247),
        };
        st.refresh();
        st
    }

    pub fn pri(&self) -> Color { Color::Rgb(self.pri_rgb.0, self.pri_rgb.1, self.pri_rgb.2) }
    pub fn sec(&self) -> Color { Color::Rgb(self.sec_rgb.0, self.sec_rgb.1, self.sec_rgb.2) }

    pub fn normalize_id(raw: &str) -> String {
        let s = raw.trim().to_ascii_lowercase();
        if s == "openclaw" || s == "claw" { "openclaw".into() }
        else if s.starts_with("agy") || s.starts_with("antigravity") { "agy".into() }
        else if s.starts_with("mcode") || s.starts_with("minimax") { "mcode".into() }
        else if s == "grok" || s == "grok-cli" { "grok".into() }
        else if s.starts_with("vibe") || s.starts_with("mistral") { "vibe".into() }
        else if s.starts_with("claude") { "claude".into() }
        else if matches!(s.as_str(), "chatgpt" | "chatgtp" | "codex" | "openai") { "chatgpt".into() }
        else if matches!(s.as_str(), "ollama" | "local" | "pi") { "ollama".into() }
        else if s.is_empty() { "agy".into() }
        else { s }
    }

    pub fn save_env(&self) {
        let content = format!(
            "AI_AGENT=\"{}\"\nOLLAMA_HOST=\"{}\"\nOLLAMA_MODEL=\"{}\"\n",
            self.active_agent, self.ollama_host, self.ollama_model
        );
        let _ = fs::write(self.state_dir.join("quake-ai.env"), content);
    }

    pub fn refresh(&mut self) {
        if let Ok(txt) = fs::read_to_string(self.state_dir.join("quake-ai.env")) {
            for line in txt.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let val = v.trim().trim_matches('"').trim_matches('\'').replace('\\', "");
                    match k.trim() {
                        "AI_AGENT" if !val.is_empty() => self.active_agent = Self::normalize_id(&val),
                        "OLLAMA_HOST" if !val.is_empty() => self.ollama_host = val,
                        "OLLAMA_MODEL" if !val.is_empty() => self.ollama_model = val,
                        _ => {}
                    }
                }
            }
        }
        self.probe_ollama();
        self.openclaw_ready = has_bin("openclaw") || has_bin("agy") || has_bin("pi");
        self.daemon_active = Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", "superagent.service"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let chat_bin = if has_bin("chatgpt") { "chatgpt" } else { "codex" };
        let oc_bin = if has_bin("openclaw") { "openclaw" } else { "agy" };
        let mut list = vec![
            ("openclaw", "autonomous", "OpenClaw Autonomous Harness (openclaw)", oc_bin, oc_bin),
            ("agy", "core", "Antigravity CLI (agy)", "agy", "agy"),
            ("mcode", "core", "MiniMax Code (mcode)", "mcode", "mcode"),
            ("grok", "core", "xAI Grok CLI (grok)", "grok", "grok"),
            ("vibe", "core", "Mistral Vibe (vibe)", "vibe", "vibe"),
            ("claude", "standard", "Claude Code (claude)", "claude", "claude"),
            ("chatgpt", "standard", "ChatGPT / Codex", chat_bin, chat_bin),
        ];
        let ol_label = format!("Ollama via Pi ({} // {})", self.ollama_host, self.ollama_model);
        let ol_cmd = format!("pi --provider ollama --model {}", self.ollama_model);

        let mut entries: Vec<AgentEntry> = list
            .drain(..)
            .map(|(id, cat, lbl, cmd, bin)| AgentEntry {
                id: id.into(),
                category: cat.into(),
                label: lbl.into(),
                cmd: cmd.into(),
                bin: bin.into(),
                installed: has_bin(bin),
                active: self.active_agent == id,
            })
            .collect();
        entries.push(AgentEntry {
            id: "ollama".into(),
            category: "local".into(),
            label: ol_label,
            cmd: ol_cmd,
            bin: "pi".into(),
            installed: self.ollama_online || has_bin("pi") || has_bin("ollama"),
            active: self.active_agent == "ollama",
        });

        if let Ok(txt) = fs::read_to_string(self.state_dir.join("ai-agents.conf")) {
            for line in txt.lines() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('#') { continue; }
                let parts: Vec<&str> = l.split('|').collect();
                if parts.len() >= 3 {
                    let cid = parts[0].trim();
                    let clbl = parts[1].trim();
                    let ccmd = parts[2].trim();
                    let cbin = ccmd.split_whitespace().next().unwrap_or(cid);
                    entries.push(AgentEntry {
                        id: cid.into(),
                        category: "custom".into(),
                        label: clbl.into(),
                        cmd: ccmd.into(),
                        bin: cbin.into(),
                        installed: has_bin(cbin),
                        active: self.active_agent == cid,
                    });
                }
            }
        }

        if let Some(act) = entries.iter().find(|e| e.active) {
            self.active_label = act.label.clone();
            self.active_cmd = act.cmd.clone();
            self.active_bin = act.bin.clone();
        }
        self.agents = entries;
    }

    pub fn probe_ollama(&mut self) {
        self.ollama_models.clear();
        self.ollama_online = false;
        let sf_ol = self.home.join(".local/bin/sf-ollama");
        let bin = if sf_ol.exists() { sf_ol.to_string_lossy().to_string() } else { "sf-ollama".into() };
        let Ok(out) = Command::new(bin).args(["probe", &self.ollama_host, &self.ollama_model]).output() else { return };
        let txt = String::from_utf8_lossy(&out.stdout);
        for line in txt.lines() {
            let p: Vec<&str> = line.split('|').collect();
            if p.first() == Some(&"OLLAMA_STATUS") && p.len() >= 4 {
                self.ollama_host = p[1].to_string();
                self.ollama_online = p[2] == "1";
                if !p[3].is_empty() {
                    self.ollama_model = p[3].to_string();
                }
            } else if p.first() == Some(&"OLLAMA_MODEL") && p.len() >= 6 {
                self.ollama_models.push(OllamaModel {
                    name: p[1].into(),
                    loaded: p[2] == "1",
                    size: p[3].into(),
                    quant: p[4].into(),
                    display: p[5].into(),
                });
            }
        }
    }
}

pub fn has_bin(name: &str) -> bool {
    let path_env = env::var("PATH").unwrap_or_default();
    let home = env::var("HOME").unwrap_or_else(|_| "/home/jeryd".into());
    let full = format!("{home}/.local/bin:{home}/.cargo/bin:{home}/.minimax-code/bin:{home}/.minimax/bin:{home}/.grok/bin:{home}/.npm-global/bin:/usr/local/bin:/usr/bin:/bin:{path_env}");
    Command::new("sh")
        .env("PATH", full)
        .args(["-c", &format!("command -v {name} >/dev/null 2>&1")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
