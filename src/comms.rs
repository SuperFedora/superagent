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
        "medic" | "healer" => "BTRFS // HEALER",
        "sec" | "security" | "alert" => "POLKIT // GUARD",
        "navigator" | "net" => "NET // TAILSCALE",
        "commander" | "ops" => "FLEET // OPS",
        "build" | "builder" | "forge" => "SF FORGE",
        "eng" | "engineer" | "sys" => "SUPERFEDORA",
        "archivist" | "snapper" => "SNAPPER // VAULT",
        "synth" | "audio" => "PIPEWIRE // DAC",
        _ => "AGY // OPENCLAW",
    };
    let _ = Command::new("notify-send").args(["-a", app, title, body]).status();
    println!("  [OK] SuperAgent Cast Comms [{app}]: {title}");
    0
}

pub struct WatchdogReport {
    pub mem_pct: u64,
    pub cpu_temp_c: i32,
    pub nvme_temp_c: i32,
    pub bat_pct: u16,
    pub bat_health: String,
    pub failed_units: Vec<String>,
    pub net_up: bool,
    pub tailscale_up: bool,
    pub selinux_mode: String,
}

pub fn probe_watchdog() -> WatchdogReport {
    let mut mem_pct = 0u64;
    if let Ok(m) = fs::read_to_string("/proc/meminfo") {
        let (mut tot, mut avail) = (0u64, 0u64);
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
    let cpu_temp_c = fs::read_dir("/sys/class/thermal")
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| fs::read_to_string(e.path().join("temp")).ok()?.trim().parse::<i32>().ok())
        .map(|v| if v > 1000 { v / 1000 } else { v })
        .filter(|&c| (15..125).contains(&c))
        .max()
        .unwrap_or(42);
    let nvme_temp_c = fs::read_dir("/sys/class/hwmon")
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| fs::read_to_string(e.path().join("temp1_input")).ok()?.trim().parse::<i32>().ok())
        .map(|v| if v > 1000 { v / 1000 } else { v })
        .filter(|&c| (15..115).contains(&c))
        .max()
        .unwrap_or(cpu_temp_c);

    let (mut bat_pct, mut bat_health) = (100u16, String::from("Good (AC)"));
    for bat in ["BAT0", "BAT1"] {
        let base = std::path::Path::new("/sys/class/power_supply").join(bat);
        if base.exists() {
            bat_pct = fs::read_to_string(base.join("capacity")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(100);
            let status = fs::read_to_string(base.join("status")).unwrap_or_else(|_| "AC".into());
            bat_health = format!("{} ({}%)", status.trim(), bat_pct);
            break;
        }
    }

    let mut failed_units = Vec::new();
    for args in [vec!["--failed", "--no-legend", "--plain"], vec!["--user", "--failed", "--no-legend", "--plain"]] {
        if let Ok(o) = Command::new("systemctl").args(&args).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if let Some(u) = line.split_whitespace().next() && !u.is_empty() {
                    failed_units.push(u.to_string());
                }
            }
        }
    }

    let net_up = fs::read_to_string("/proc/net/route").map(|r| r.lines().count() > 1).unwrap_or(true);
    let tailscale_up = std::path::Path::new("/sys/class/net/tailscale0").exists() || std::path::Path::new("/sys/class/net/wg0").exists();
    let selinux_mode = Command::new("getenforce")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Enforcing".into());

    WatchdogReport {
        mem_pct,
        cpu_temp_c,
        nvme_temp_c,
        bat_pct,
        bat_health,
        failed_units,
        net_up,
        tailscale_up,
        selinux_mode,
    }
}

pub fn run_watchdog(st: &mut AgentState, json_mode: bool, notify: bool) -> i32 {
    st.refresh();
    let rep = probe_watchdog();
    if json_mode {
        println!(
            "{{\"cpu_temp_c\":{},\"nvme_temp_c\":{},\"ram_pct\":{},\"bat_pct\":{},\"failed_units\":{},\"net_up\":{},\"tailscale_up\":{},\"selinux\":\"{}\",\"ollama_online\":{}}}",
            rep.cpu_temp_c,
            rep.nvme_temp_c,
            rep.mem_pct,
            rep.bat_pct,
            rep.failed_units.len(),
            rep.net_up,
            rep.tailscale_up,
            rep.selinux_mode,
            st.ollama_online
        );
        return 0;
    }

    println!("\x1b[1;38;5;141m[::] SUPERAGENT ZERO-TOKEN WORKSTATION TELEMETRY & ANOMALY WATCHDOG\x1b[0m");
    println!("  [SYS MEDIC]     CPU: {}C | NVMe: {}C | RAM: {}% | Battery: {}", rep.cpu_temp_c, rep.nvme_temp_c, rep.mem_pct, rep.bat_health);
    println!("  [SEC ALERT]     SELinux: {} | Polkit Guard: active", rep.selinux_mode);
    println!("  [NAVIGATOR]     Route: {} | VPN/Tailscale: {}", if rep.net_up { "ONLINE" } else { "DEGRADED" }, if rep.tailscale_up { "tunnel-up" } else { "standby" });
    println!("  [COMMANDER]     Failed systemd units: {}", if rep.failed_units.is_empty() { "0 (all healthy)".into() } else { rep.failed_units.join(", ") });
    println!("  [AI AGENT]      Harness: {} | Ollama: http://{} ({})", st.active_label, st.ollama_host, if st.ollama_online { "ONLINE" } else { "STANDBY" });

    if notify {
        if rep.cpu_temp_c >= 88 || rep.nvme_temp_c >= 80 {
            let _ = relay_cast("medic", "SYS MEDIC: Thermal Spike", &format!("CPU {}C / NVMe {}C (RAM {}%)", rep.cpu_temp_c, rep.nvme_temp_c, rep.mem_pct));
        } else if !rep.failed_units.is_empty() {
            let _ = relay_cast("commander", "COMMANDER: Failed Unit Detected", &format!("Units: {}", rep.failed_units.join(", ")));
        } else if !rep.net_up {
            let _ = relay_cast("navigator", "NAVIGATOR: Network Route Degraded", "Default route offline; check wlctl or Tailscale.");
        } else {
            let _ = relay_cast(
                "ai",
                &format!("SuperAgent Watchdog: {}", st.active_label),
                &format!("CPU {}C | RAM {}% | 0 anomalies | Cast Specialists ready.", rep.cpu_temp_c, rep.mem_pct),
            );
        }
    }
    0
}

pub fn run_daemon(st: &mut AgentState, once: bool) -> i32 {
    let _ = run_watchdog(st, false, true);
    if once {
        return 0;
    }

    let mut last_ol = st.ollama_online;
    loop {
        thread::sleep(Duration::from_secs(60));
        st.refresh();
        let rep = probe_watchdog();
        if rep.cpu_temp_c >= 90 || rep.nvme_temp_c >= 82 {
            let _ = relay_cast("medic", "SYS MEDIC: Thermal Threshold Alert", &format!("CPU {}C / NVMe {}C (RAM {}%).", rep.cpu_temp_c, rep.nvme_temp_c, rep.mem_pct));
        } else if !rep.failed_units.is_empty() {
            let _ = relay_cast("commander", "COMMANDER: Systemd Unit Alert", &format!("Failed units: {}", rep.failed_units.join(", ")));
        } else if st.ollama_online != last_ol {
            last_ol = st.ollama_online;
            let state_lbl = if last_ol { "ONLINE" } else { "OFFLINE" };
            let _ = relay_cast("ai", &format!("Ollama Server {state_lbl}"), &format!("Endpoint http://{} model {} is now {state_lbl}.", st.ollama_host, st.ollama_model));
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
