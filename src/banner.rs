use colored::*;

/// Prints the stylized Claude-Code-style ASCII banner with an Electric Cyan to Orchid Purple gradient.
pub fn print_banner() {
    let lines = [
        "   ██████╗   ██████╗  ██╗      ███████╗ ███╗   ███╗",
        "  ██╔════╝  ██╔═══██╗ ██║      ██╔════╝ ████╗ ████║",
        "  ██║  ███╗ ██║   ██║ ██║      █████╗   ██╔████╔██║",
        "  ██║   ██║ ██║   ██║ ██║      ██╔══╝   ██║╚██╔╝██║",
        "  ╚██████╔╝ ╚██████╔╝ ███████╗ ███████╗ ██║ ╚═╝ ██║",
        "   ╚═════╝   ╚═════╝  ╚══════╝ ╚══════╝ ╚═╝     ╚═╝",
    ];

    // Electric Cyan to Royal Lavender gradient theme
    let colors = [
        "\x1b[38;5;51m",  // Electric Cyan
        "\x1b[38;5;87m",  // Soft Aqua
        "\x1b[38;5;123m", // Ice Blue
        "\x1b[38;5;147m", // Periwinkle Blue
        "\x1b[38;5;141m", // Lavender Violet
        "\x1b[38;5;135m", // Deep Orchid
    ];

    println!();
    for (i, line) in lines.iter().enumerate() {
        println!("{}{}\x1b[0m", colors[i % colors.len()], line);
    }
    println!(
        "   {} \x1b[38;5;242m•\x1b[0m {}",
        "Minecraft Server Container Engine".dimmed(),
        format!("v{}", env!("CARGO_PKG_VERSION")).cyan()
    );
    println!();
}

/// Prints the welcome view when `golem` is launched without arguments.
pub fn print_welcome(active_servers: Option<(usize, usize)>) {
    print_banner();

    if let Some((running, total)) = active_servers {
        if total > 0 {
            println!(
                "   {} Active Servers: {} running, {} total (type {} to view)",
                "●".green(),
                running.to_string().bold().green(),
                total.to_string().bold().white(),
                "golem ls".yellow()
            );
            println!();
        }
    }

    println!("{}", "   Usage:".bold());
    println!("     {} {}", "golem".cyan().bold(), "<COMMAND> [OPTIONS]".white());
    println!();

    println!("{}", "   Commands:".bold());
    println!("     {:<12} {}", "create".cyan(), "Spin up a new Minecraft server container");
    println!("     {:<12} {}", "ls".cyan(), "List all active and stopped servers (alias: list)");
    println!("     {:<12} {}", "stats".cyan(), "Inspect live server health, resources & logs (alias: dash)");
    println!("     {:<12} {}", "run".cyan(), "Start or resume a stopped server (alias: start)");
    println!("     {:<12} {}", "stop".cyan(), "Gracefully stop a running server");
    println!("     {:<12} {}", "rm".cyan(), "Remove a server (and optionally purge world data)");
    println!("     {:<12} {}", "logs".cyan(), "View or stream live server console logs (-f to follow)");
    println!("     {:<12} {}", "backup".cyan(), "Create a compressed world snapshot (.tar.gz)");
    println!("     {:<12} {}", "restore".cyan(), "Restore a server world from a backup archive");
    println!("     {:<12} {}", "help".cyan(), "Print help information");
    println!();

    println!("{}", "   Examples:".bold());
    println!("     {}", "golem create survival --port 8000".dimmed());
    println!("     {}", "golem ls".dimmed());
    println!("     {}", "golem stats survival".dimmed());
    println!("     {}", "golem stop survival".dimmed());
    println!();
}
