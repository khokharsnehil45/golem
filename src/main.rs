mod cli;
mod docker;
mod network;
mod stats;

use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use docker::CreateServerOptions;

fn main() {
    let cli = Cli::parse();

    // Verify Docker daemon before running any command
    if let Err(e) = docker::ensure_docker_available() {
        eprintln!("{} {}", "Error:".bold().red(), e);
        std::process::exit(1);
    }

    match cli.command {
        Commands::Create {
            server_name,
            port,
            server_type,
            memory,
            online_mode,
        } => {
            println!("{}", "🗿 Golem - Minecraft Server Launcher".bold().cyan());
            println!();

            // Step 1: Check container name availability
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
                        format!("docker rm -f {}", server_name).bold()
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

            // Step 2: Check port availability
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

            // Step 3: Launch container
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
                    println!("  • View stats:          {}", format!("golem stats {}", server_name).bold().yellow());
                    println!("  • Stop server:         {}", format!("golem stop {}", server_name).white());
                    println!("  • Run/Start server:    {}", format!("golem run {}", server_name).white());
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

        Commands::Run { server_name } => {
            println!("{}", "🗿 Golem - Minecraft Server Launcher".bold().cyan());

            // Check if container exists
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

            // Check current status
            let status = docker::get_container_status(&server_name).unwrap_or_default();
            if status == "running" {
                println!(
                    "{} Server '{}' is already running!",
                    "ℹ".cyan().bold(),
                    server_name.yellow()
                );
            } else {
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
            println!();
        }

        Commands::Stop { server_name } => {
            println!("{}", "🗿 Golem - Minecraft Server Launcher".bold().cyan());

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

        Commands::Stats { server_name } => {
            stats::show_simple_stats(&server_name);
        }
    }
}
