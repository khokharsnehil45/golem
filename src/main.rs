mod banner;
mod cli;
mod docker;
mod network;
mod stats;
mod validation;

use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use docker::CreateServerOptions;
use std::io::{self, Write};

fn main() {
    let cli = Cli::parse();

    // Verify Docker daemon before running any command
    if let Err(e) = docker::ensure_docker_available() {
        banner::print_banner();
        eprintln!("{} {}", "Error:".bold().red(), e);
        std::process::exit(1);
    }

    match cli.command {
        None => {
            // When user runs bare `golem` command
            let server_stats = docker::list_minecraft_servers().ok().map(|servers| {
                let total = servers.len();
                let running = servers.iter().filter(|s| s.status == "running").count();
                (running, total)
            });
            banner::print_welcome(server_stats);
        }

        Some(Commands::Create {
            server_name,
            port,
            server_type,
            memory,
            online_mode,
        }) => {
            banner::print_banner();

            // Step 1: Validate inputs
            if let Err(e) = validation::validate_server_name(&server_name) {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }

            if let Err(e) = validation::validate_port(port) {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }

            if let Err(e) = validation::validate_memory(&memory) {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }

            // Step 2: Check container name availability
            print!("{} Checking container name availability... ", "•".cyan());
            match docker::container_exists(&server_name) {
                Ok(true) => {
                    println!("{}", "EXISTS".bold().red());
                    eprintln!(
                        "\n{} A container named '{}' already exists.",
                        "Error:".bold().red(),
                        server_name.yellow()
                    );
                    eprintln!(
                        "Choose another name or delete it using: {}",
                        format!("golem rm -f {}", server_name).bold()
                    );
                    std::process::exit(1);
                }
                Ok(false) => {
                    println!("{}", "OK".bold().green());
                }
                Err(e) => {
                    println!("{}", "WARNING".bold().yellow());
                    eprintln!("Warning while checking existing containers: {}", e);
                }
            }

            // Step 3: Check port availability
            print!("{} Checking port {} availability... ", "•".cyan(), port.to_string().yellow());
            if !network::is_port_available(port) {
                println!("{}", "BUSY".bold().red());
                eprintln!(
                    "\n{} Port {} is already in use by another service on your machine.",
                    "Error:".bold().red(),
                    port
                );
                eprintln!("Please specify a different port using {}", "--port <NUMBER>".yellow());
                std::process::exit(1);
            }
            println!("{}", "OK".bold().green());

            // Step 4: Launch container
            println!("\n{} Spinning up Minecraft server '{}'...", "▶".bold().blue(), server_name.bold());
            let opts = CreateServerOptions {
                server_name: &server_name,
                port,
                server_type: &server_type,
                memory: &memory,
                online_mode,
            };

            match docker::run_minecraft_container(&opts) {
                Ok(container_id) => {
                    let short_id = if container_id.len() >= 12 {
                        &container_id[..12]
                    } else {
                        &container_id
                    };

                    println!("\n{}", "✔ Server container created successfully!".bold().green());
                    println!("{}", "──────────────────────────────────────────────────────".dimmed());
                    println!("  {:<18} {}", "Server Name:".bold(), server_name.cyan());
                    println!("  {:<18} {}", "Container ID:".bold(), short_id);
                    println!("  {:<18} {}", "Server Engine:".bold(), server_type);
                    println!("  {:<18} {}", "Allocated RAM:".bold(), memory);
                    println!("  {:<18} {}", "Persistent Data:".bold(), format!("golem_{}_data", server_name));
                    println!("  {:<18} {}", "Online Mode:".bold(), if online_mode { "True (Mojang Auth)" } else { "False (Offline/Custom)" });
                    println!("{}", "──────────────────────────────────────────────────────".dimmed());

                    // Connection IPs
                    println!("\n{}", "🎮 How to connect in Minecraft (Direct Connection / Add Server):".bold().yellow());
                    if port == 25565 {
                        println!("  • Local (this PC):     {}", "localhost".bold().green());
                    } else {
                        println!("  • Local (this PC):     {}", format!("localhost:{}", port).bold().green());
                    }

                    if let Some(lan_ip) = network::get_local_ip() {
                        if port == 25565 {
                            println!("  • LAN (same Wi-Fi):    {}", lan_ip.bold().green());
                        } else {
                            println!("  • LAN (same Wi-Fi):    {}", format!("{}:{}", lan_ip, port).bold().green());
                        }
                    }

                    println!("\n{}", "📋 Server management:".bold().cyan());
                    println!("  • View all servers:    {}", "golem ls".bold().white());
                    println!("  • View stats:          {}", format!("golem stats {}", server_name).bold().yellow());
                    println!("  • Stop server:         {}", format!("golem stop {}", server_name).white());
                    println!("  • Run/Start server:    {}", format!("golem run {}", server_name).white());
                    println!("  • Remove server:       {}", format!("golem rm {}", server_name).white());
                    println!("  • Follow raw logs:     {}", format!("docker logs -f {}", server_name).white());
                    println!();
                }
                Err(e) => {
                    println!("{}", "FAILED".bold().red());
                    eprintln!("\n{} Failed to start container:\n{}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Ls) => {
            banner::print_banner();

            match docker::list_minecraft_servers() {
                Ok(servers) if servers.is_empty() => {
                    println!("{}", "No Minecraft server containers found.".yellow());
                    println!(
                        "Create your first server with: {}",
                        "golem create <name> --port <port>".bold().green()
                    );
                    println!();
                }
                Ok(servers) => {
                    println!(
                        "{}",
                        format!("Active Minecraft Servers ({} total):\n", servers.len()).bold().white()
                    );

                    let max_name_len = servers
                        .iter()
                        .map(|s| s.name.len())
                        .max()
                        .unwrap_or(4)
                        .max(16);

                    // Header
                    println!(
                        "  {:<name_w$}  {:<14}  {:<8}  {:<8}  {:<6}  {}",
                        "NAME",
                        "STATUS",
                        "PORT",
                        "ENGINE",
                        "RAM",
                        "LOCAL ADDRESS",
                        name_w = max_name_len
                    );
                    println!(
                        "  {}",
                        "─".repeat(max_name_len + 56).dimmed()
                    );

                    for s in servers {
                        let padded_name = format!("{:<width$}", s.name, width = max_name_len);

                        // Visual alignment: "🟢 Running " and "🔴 Stopped " are each exactly 14 display chars
                        let status_display = if s.status == "running" {
                            "🟢 Running    ".green().bold()
                        } else if s.status == "exited" {
                            "🔴 Stopped    ".red().bold()
                        } else {
                            format!("{:<14}", format!("🟡 {}", s.status)).yellow().bold()
                        };

                        let port_raw = s.port.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
                        let padded_port = format!("{:<8}", port_raw);

                        let padded_type = format!("{:<8}", s.server_type);
                        let padded_mem = format!("{:<6}", s.memory);

                        let local_addr = match s.port {
                            Some(25565) => "localhost".to_string(),
                            Some(p) => format!("localhost:{}", p),
                            None => "-".to_string(),
                        };

                        println!(
                            "  {}  {}  {}  {}  {}  {}",
                            padded_name.cyan().bold(),
                            status_display,
                            padded_port.yellow(),
                            padded_type.white(),
                            padded_mem.white(),
                            local_addr.white()
                        );
                    }

                    println!(
                        "  {}",
                        "─".repeat(max_name_len + 56).dimmed()
                    );
                    println!("\n{}", "📋 Quick Commands:".bold().cyan());
                    println!("  • View stats:        {}", "golem stats <name>".white());
                    println!("  • Start a server:    {}", "golem run <name>".white());
                    println!("  • Stop a server:     {}", "golem stop <name>".white());
                    println!("  • Remove a server:   {}", "golem rm <name>".white());
                    println!();
                }
                Err(e) => {
                    eprintln!("{} Failed to list servers: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Run { server_name }) => {
            banner::print_banner();

            // 1. Check if container exists
            match docker::container_exists(&server_name) {
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

            // 2. Check current status
            let status = docker::get_container_status(&server_name).unwrap_or_default();
            if status == "running" {
                println!(
                    "{} Server '{}' is already running!",
                    "ℹ".cyan().bold(),
                    server_name.yellow()
                );
            } else {
                // 3. Proactively check if port is currently free before starting
                if let Some(port) = docker::get_container_port(&server_name) {
                    if !network::is_port_available(port) {
                        eprintln!(
                            "\n{} Cannot start server '{}': Port {} is already in use by another service on your machine.",
                            "Error:".bold().red(),
                            server_name.yellow(),
                            port
                        );
                        std::process::exit(1);
                    }
                }

                print!("{} Starting server '{}'... ", "▶".cyan().bold(), server_name.bold());
                match docker::start_container(&server_name) {
                    Ok(_) => println!("{}", "OK".bold().green()),
                    Err(e) => {
                        println!("{}", "FAILED".bold().red());
                        eprintln!("\n{} Failed to start server: {}", "Error:".bold().red(), e);
                        std::process::exit(1);
                    }
                }
            }

            println!("\n{}", "✔ Server is active!".bold().green());
            if let Some(port) = docker::get_container_port(&server_name) {
                println!("{}", "🎮 Connection details:".bold().yellow());
                if port == 25565 {
                    println!("  • Local (this PC):     {}", "localhost".bold().green());
                } else {
                    println!("  • Local (this PC):     {}", format!("localhost:{}", port).bold().green());
                }

                if let Some(lan_ip) = network::get_local_ip() {
                    if port == 25565 {
                        println!("  • LAN (same Wi-Fi):    {}", lan_ip.bold().green());
                    } else {
                        println!("  • LAN (same Wi-Fi):    {}", format!("{}:{}", lan_ip, port).bold().green());
                    }
                }
            }

            println!("\n{}", "📋 Management:".bold().cyan());
            println!("  • View stats:          {}", format!("golem stats {}", server_name).bold().yellow());
            println!("  • Stop server:         {}", format!("golem stop {}", server_name).white());
            println!("  • Remove server:       {}", format!("golem rm {}", server_name).white());
            println!();
        }

        Some(Commands::Stop { server_name }) => {
            banner::print_banner();

            // Check if container exists
            match docker::container_exists(&server_name) {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!(
                        "{} Server '{}' does not exist.",
                        "Error:".bold().red(),
                        server_name.yellow()
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{} Failed to check server: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }

            // Check if already stopped
            let status = docker::get_container_status(&server_name).unwrap_or_default();
            if status != "running" {
                println!(
                    "{} Server '{}' is already stopped (status: {}).",
                    "ℹ".cyan().bold(),
                    server_name.yellow(),
                    status
                );
                return;
            }

            print!("{} Stopping server '{}'... ", "■".yellow().bold(), server_name.bold());
            match docker::stop_container(&server_name) {
                Ok(_) => {
                    println!("{}", "OK".bold().green());
                    println!(
                        "\n{} Server '{}' has been stopped safely.",
                        "✔".bold().green(),
                        server_name.cyan()
                    );
                    println!(
                        "  Restart it anytime with: {}",
                        format!("golem run {}", server_name).bold().yellow()
                    );
                    println!();
                }
                Err(e) => {
                    println!("{}", "FAILED".bold().red());
                    eprintln!("\n{} Failed to stop server: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Rm {
            server_name,
            purge,
            force,
        }) => {
            banner::print_banner();

            // 1. Check if container exists
            match docker::container_exists(&server_name) {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!(
                        "{} Server '{}' does not exist.",
                        "Error:".bold().red(),
                        server_name.yellow()
                    );
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{} Failed to check server: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }

            let volume_name = format!("golem_{}_data", server_name);
            let has_volume = docker::volume_exists(&volume_name);

            // 2. Confirmation prompt if not forced
            if !force {
                if purge {
                    print!(
                        "{} Are you sure you want to permanently delete container '{}' AND world data volume '{}'? [y/N]: ",
                        "⚠️".yellow(),
                        server_name.bold().yellow(),
                        volume_name.bold().red()
                    );
                } else {
                    print!(
                        "{} Are you sure you want to delete container '{}'? (World data will be kept) [y/N]: ",
                        "⚠️".yellow(),
                        server_name.bold().yellow()
                    );
                }
                io::stdout().flush().unwrap();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() || {
                    let t = input.trim().to_lowercase();
                    t != "y" && t != "yes"
                } {
                    println!("{}", "Deletion cancelled.".dimmed());
                    return;
                }
            }

            // 3. Remove container
            print!("{} Removing container '{}'... ", "•".cyan(), server_name.bold());
            match docker::remove_container(&server_name) {
                Ok(_) => println!("{}", "OK".bold().green()),
                Err(e) => {
                    println!("{}", "FAILED".bold().red());
                    eprintln!("\n{} Failed to remove container: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            }

            // 4. Purge volume if requested
            if purge {
                if has_volume {
                    print!("{} Removing volume '{}'... ", "•".cyan(), volume_name.bold());
                    match docker::remove_volume(&volume_name) {
                        Ok(_) => println!("{}", "OK".bold().green()),
                        Err(e) => {
                            println!("{}", "FAILED".bold().red());
                            eprintln!("Warning: Failed to delete volume: {}", e);
                        }
                    }
                }
            } else if has_volume {
                println!(
                    "{} World data volume '{}' was preserved.",
                    "ℹ".cyan(),
                    volume_name.dimmed()
                );
                println!(
                    "  (To delete world data too, run: {})",
                    format!("golem rm {} --purge", server_name).dimmed()
                );
            }

            println!("\n{} Server '{}' successfully removed!\n", "✔".bold().green(), server_name.cyan());
        }

        Some(Commands::Stats { server_name }) => {
            banner::print_banner();
            stats::show_simple_stats(&server_name);
        }
    }
}
