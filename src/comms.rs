// Copyright 2026 SuperFedora Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::state::AgentState;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn relay_cast(role: &str, title: &str, body: &str) -> i32 {
    if let Ok(st) = Command::new("supernotify").args(["--cast", role, title, body]).status()
        && st.success()
    {
        return 0;
    }
    let app = match role.to_ascii_lowercase().as_str() {
        "sec" | "security" | "alert" => "POLKIT // GUARD",
        "build" | "builder" | "forge" => "SF FORGE",
        "eng" | "engineer" | "sys" => "SUPERFEDORA",
        _ => "AGY // OPENCLAW",
    };
    let _ = Command::new("notify-send").args(["-a", app, title, body]).status();
    println!("  [OK] SuperAgent Cast Comms [{app}]: {title}");
    0
}

pub fn probe_vitals_summary() -> (u64, i32) {
    let mut mem_pct = 0u64;
    if let Ok(m) = fs::read_to_string("/proc/meminfo") {
        let mut tot = 0u64;
        let mut avail = 0u64;
        for l in m.lines() {
            if let Some(v) = l.strip_prefix("MemTotal:") {
                tot = v.split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0);
            } else if let Some(v) = l.strip_prefix("MemAvailable:") {
                avail = v.split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0);
            }
        }
        if tot > 0 {
            mem_pct = 100u64.saturating_sub((avail * 100) / tot);
        }
    }
    let raw_t = fs::read_to_string("/sys/class/thermal/thermal_zone6/temp")
        .or_else(|_| fs::read_to_string("/sys/class/thermal/thermal_zone0/temp"))
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(42000);
    let temp_c = if raw_t > 1000 { raw_t / 1000 } else { raw_t };
    (mem_pct, temp_c)
}

pub fn run_daemon(st: &mut AgentState, once: bool) -> i32 {
    st.refresh();
    let (mem_pct, temp_c) = probe_vitals_summary();
    let ol_str = if st.ollama_online {
        format!("Ollama ONLINE ({} // {})", st.ollama_host, st.ollama_model)
    } else {
        format!("Ollama standby ({})", st.ollama_host)
    };
    let summary = format!("SuperAgent Active: {}", st.active_label);
    let body = format!("{ol_str} | RAM: {mem_pct}% | Temp: {temp_c} C | Cast Comms link ready.");
    let _ = relay_cast("ai", &summary, &body);

    if once {
        return 0;
    }

    let mut last_ol = st.ollama_online;
    loop {
        thread::sleep(Duration::from_secs(60));
        st.refresh();
        let (mem, temp) = probe_vitals_summary();
        if temp >= 90 {
            let _ = relay_cast(
                "security",
                "Thermal Threshold Alert",
                &format!("CPU package temperature reached {temp} C (RAM {mem}%)."),
            );
        } else if st.ollama_online != last_ol {
            last_ol = st.ollama_online;
            let state_lbl = if last_ol { "ONLINE" } else { "OFFLINE" };
            let _ = relay_cast(
                "ai",
                &format!("Ollama Server {state_lbl}"),
                &format!("Endpoint http://{} model {} is now {state_lbl}.", st.ollama_host, st.ollama_model),
            );
        }
    }
}

pub fn ask_agent(st: &AgentState, prompt: &str) -> i32 {
    let q = prompt.trim();
    if q.is_empty() {
        println!("Please provide a prompt.");
        return 0;
    }
    let runner = st.agent_root.join("lib/quake-runner.sh");
    if runner.exists() {
        return Command::new(runner)
            .arg("ask")
            .arg(q)
            .status()
            .map(|s| s.code().unwrap_or(0))
            .unwrap_or(1);
    }
    let sf_ol = st.home.join(".local/bin/sf-ollama");
    Command::new(sf_ol)
        .args(["ask", &st.ollama_host, &st.ollama_model, q])
        .status()
        .map(|s| s.code().unwrap_or(0))
        .unwrap_or(1)
}

pub fn exec_quake_runner(st: &AgentState, args: &[String]) -> i32 {
    let runner = st.agent_root.join("lib/quake-runner.sh");
    let fallback = st.home.join(".config/hypr/scripts/quake-ai.sh");
    let target = if runner.exists() { runner } else { fallback };
    Command::new(target)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .map(|s| s.code().unwrap_or(0))
        .unwrap_or(1)
}
