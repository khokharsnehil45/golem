use colored::*;

use crate::docker;
use crate::network;

pub fn show_simple_stats(server_name: &str) {
    println!("{}", format!("Server Status: {}", server_name).bold().cyan());

    // 1. Check if container exists
    match docker::container_exists(server_name) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "{} Server '{}' does not exist.\nCreate it first using: {}",
                "Error:".bold().red(),
                server_name.yellow(),
                format!("golem create {} --port <PORT>", server_name).bold()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{} Failed to check server: {}", "Error:".bold().red(), e);
            std::process::exit(1);
        }
    }

    // 2. Fetch container status
    let status = docker::get_container_status(server_name).unwrap_or_else(|_| "unknown".to_string());
    let is_running = status == "running";

    let status_display = if is_running {
        format!("🟢 RUNNING ({})", status).bold().green()
    } else {
        format!("🔴 STOPPED ({})", status).bold().red()
    };

    let port = docker::get_container_port(server_name);
    let port_str = port.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());

    let local_addr = match port {
        Some(25565) => "localhost".to_string(),
        Some(p) => format!("localhost:{}", p),
        None => "N/A".to_string(),
    };

    let lan_addr = match (network::get_local_ip(), port) {
        (Some(ip), Some(25565)) => ip,
        (Some(ip), Some(p)) => format!("{}:{}", ip, p),
        _ => "N/A".to_string(),
    };

    println!("{}", "──────────────────────────────────────────────────────".dimmed());
    println!("  {:<18} {}", "• Status:".bold(), status_display);
    println!("  {:<18} {}", "• Host Port:".bold(), port_str);
    println!("  {:<18} {}", "• Local Connect:".bold(), local_addr.green().bold());
    println!("  {:<18} {}", "• LAN Connect:".bold(), lan_addr.cyan().bold());
    println!("  {:<18} {}", "• Storage Volume:".bold(), format!("golem_{}_data", server_name).dimmed());

    // 3. Stats if running
    if is_running {
        if let Ok((cpu, mem)) = docker::get_container_stats(server_name) {
            println!("  {:<18} {}", "• CPU Usage:".bold(), format!("{} {}", cpu.yellow().bold(), "(Multi-core load)".dimmed()));

            let (current_mem, host_total) = if let Some((c, h)) = mem.split_once('/') {
                (c.trim(), h.trim())
            } else {
                (mem.trim(), "")
            };

            let mem_display = if let Some(alloc) = docker::get_container_env(server_name, "MEMORY") {
                if !host_total.is_empty() {
                    format!("{} / {} (Allocated Heap) {}", current_mem.magenta().bold(), alloc.cyan().bold(), format!("[Host Total: {}]", host_total).dimmed())
                } else {
                    format!("{} / {} (Allocated Heap)", current_mem.magenta().bold(), alloc.cyan().bold())
                }
            } else {
                mem.magenta().bold().to_string()
            };

            println!("  {:<18} {}", "• Memory Usage:".bold(), mem_display);
        }
    }

    // 4. Configured settings if available
    if let Some(max_p) = docker::get_container_env(server_name, "MAX_PLAYERS") {
        println!("  {:<18} {}", "• Max Players:".bold(), max_p.white());
    }
    if let Some(vd) = docker::get_container_env(server_name, "VIEW_DISTANCE") {
        println!("  {:<18} {}", "• View Distance:".bold(), format!("{} chunks", vd).white());
    }
    println!("{}", "──────────────────────────────────────────────────────".dimmed());

    // 4. Recent logs if available
    if let Ok(logs) = docker::get_container_logs(server_name, 6) {
        if !logs.is_empty() {
            println!("\n{}", "📜 Recent Logs (last few lines):".bold().white());
            for line in logs {
                if line.contains("WARN") {
                    println!("  {}", line.yellow());
                } else if line.contains("ERROR") || line.contains("Exception") {
                    println!("  {}", line.red());
                } else {
                    println!("  {}", line.white());
                }
            }
        }
    }

    // 5. Quick commands
    println!("\n{}", "📋 Quick Commands:".bold().cyan());
    if is_running {
        println!("  • Stop server:    {}", format!("golem stop {}", server_name).white());
        println!("  • Follow logs:    {}", format!("docker logs -f {}", server_name).white());
    } else {
        println!("  • Run server:     {}", format!("golem run {}", server_name).white());
    }
    println!();
}
