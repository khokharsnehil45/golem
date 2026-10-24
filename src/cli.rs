use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "golem",
    version,
    about = "A fast CLI tool to spin up and manage Minecraft server containers using Docker",
    long_about = "Golem lets you effortlessly spin up isolated, persistent Minecraft servers using Docker containers."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create and start a new Minecraft server container
    Create {
        /// Name of the server and Docker container
        server_name: String,

        /// Host port to expose the Minecraft server on (default: 25565)
        #[arg(short, long, default_value_t = 25565)]
        port: u16,

        /// Server type (e.g. PAPER, SPIGOT, FORGE, FABRIC, VANILLA)
        #[arg(long, default_value = "PAPER")]
        server_type: String,

        /// Memory allocation (e.g. 2G, 4G)
        #[arg(short, long, default_value = "2G")]
        memory: String,

        /// Set online mode (set to true for Mojang auth, false for offline/custom launchers)
        #[arg(long, default_value_t = false)]
        online_mode: bool,
    },

    /// Run / start an existing stopped Minecraft server container
    #[command(alias = "start")]
    Run {
        /// Name of the server container to run
        server_name: String,
    },

    /// Stop a running Minecraft server container
    Stop {
        /// Name of the server container to stop
        server_name: String,
    },

    /// View simple server status, port, resource usage, and recent logs
    #[command(alias = "dash", alias = "status")]
    Stats {
        /// Name of the server container to inspect
        server_name: String,
    },
}
