use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::docker;

/// Returns the default backup directory: ~/.golem/backups
pub fn get_default_backup_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".golem").join("backups")
    } else {
        PathBuf::from(".golem").join("backups")
    }
}

/// Creates a compressed snapshot of the server world volume.
pub fn create_backup(
    server_name: &str,
    custom_dir: Option<&str>,
) -> Result<(PathBuf, u64), String> {
    let volume_name = format!("golem_{}_data", server_name);

    // 1. Verify that the volume exists
    if !docker::volume_exists(&volume_name) {
        return Err(format!(
            "World volume '{}' does not exist. Has server '{}' been created?",
            volume_name, server_name
        ));
    }

    // 2. If the container is currently running, flush world chunks to disk
    if let Ok(status) = docker::get_container_status(server_name) {
        if status == "running" {
            print!("{} Flushing in-memory world chunks to disk... ", "•".cyan());
            io::stdout().flush().unwrap();
            let _ = Command::new("docker")
                .args(["exec", server_name, "rcon-cli", "save-all"])
                .output();
            println!("{}", "OK".bold().green());
        }
    }

    // 3. Resolve destination directory
    let backup_dir = match custom_dir {
        Some(d) => PathBuf::from(d),
        None => get_default_backup_dir(),
    };

    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let canonical_backup_dir = backup_dir
        .canonicalize()
        .map_err(|e| format!("Invalid backup directory path: {}", e))?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H%M%S");
    let filename = format!("{}_backup_{}.tar.gz", server_name, timestamp);
    let target_file_path = canonical_backup_dir.join(&filename);

    print!("{} Compressing server data into archive... ", "•".cyan());
    io::stdout().flush().unwrap();

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let user_arg = format!("{}:{}", uid, gid);
    let volume_mount = format!("{}:/source:ro", volume_name);
    let backup_mount = format!("{}:/backup", canonical_backup_dir.display());

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--user",
            &user_arg,
            "-v",
            &volume_mount,
            "-v",
            &backup_mount,
            "alpine",
            "tar",
            "-czf",
            &format!("/backup/{}", filename),
            "-C",
            "/source",
            ".",
        ])
        .output()
        .map_err(|e| format!("Failed to run backup container: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Backup failed: {}", err.trim()));
    }

    println!("{}", "OK".bold().green());

    let file_size = fs::metadata(&target_file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok((target_file_path, file_size))
}

/// Restores a server world from a .tar.gz archive.
pub fn restore_backup(
    server_name: &str,
    backup_file_str: &str,
    force: bool,
) -> Result<(), String> {
    let backup_path = Path::new(backup_file_str);
    if !backup_path.exists() {
        return Err(format!("Backup file '{}' does not exist.", backup_file_str));
    }

    let canonical_file = backup_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve backup file path: {}", e))?;

    let backup_dir = canonical_file
        .parent()
        .ok_or_else(|| "Invalid backup file path.".to_string())?;
    let file_name = canonical_file
        .file_name()
        .ok_or_else(|| "Invalid file name.".to_string())?
        .to_str()
        .ok_or_else(|| "Invalid Unicode in file name.".to_string())?;

    let volume_name = format!("golem_{}_data", server_name);

    // 1. Check if container is running; if so, stop it first
    let was_running = if let Ok(status) = docker::get_container_status(server_name) {
        if status == "running" {
            println!(
                "{} Server '{}' is currently running. Stopping server before restoring...",
                "ℹ".cyan(),
                server_name.yellow()
            );
            docker::stop_container(server_name)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    // 2. Confirmation prompt if not forced
    if !force {
        print!(
            "{} Are you sure you want to restore '{}'?\n   This will {} all existing world data in volume '{}'! [y/N]: ",
            "⚠️".yellow(),
            file_name.bold().yellow(),
            "OVERWRITE".bold().red(),
            volume_name.bold().cyan()
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || {
            let t = input.trim().to_lowercase();
            t != "y" && t != "yes"
        } {
            println!("{}", "Restore cancelled.".dimmed());
            return Ok(());
        }
    }

    // 3. Ensure target volume exists
    if !docker::volume_exists(&volume_name) {
        let _ = Command::new("docker")
            .args(["volume", "create", &volume_name])
            .output();
    }

    print!("{} Restoring world files into volume '{}'... ", "•".cyan(), volume_name.bold());
    io::stdout().flush().unwrap();

    let volume_mount = format!("{}:/target", volume_name);
    let backup_mount = format!("{}:/backup:ro", backup_dir.display());
    let restore_script = format!("rm -rf /target/* && tar -xzf /backup/{} -C /target", file_name);

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &volume_mount,
            "-v",
            &backup_mount,
            "alpine",
            "sh",
            "-c",
            &restore_script,
        ])
        .output()
        .map_err(|e| format!("Failed to run restore container: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Restore failed: {}", err.trim()));
    }

    println!("{}", "OK".bold().green());
    println!(
        "\n{} Server '{}' successfully restored from backup!",
        "✔".bold().green(),
        server_name.cyan()
    );

    if was_running {
        println!(
            "  Restart your server with: {}",
            format!("golem run {}", server_name).bold().yellow()
        );
    }
    println!();

    Ok(())
}

/// Lists all backup snapshots available in the default backup directory.
pub fn list_backups(server_filter: Option<&str>) -> Result<(), String> {
    let backup_dir = get_default_backup_dir();
    if !backup_dir.exists() {
        println!("{}", "No backups found (directory does not exist yet).".yellow());
        return Ok(());
    }

    let mut backups = Vec::new();
    let entries = fs::read_dir(&backup_dir)
        .map_err(|e| format!("Failed to read backup directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".tar.gz") {
                    if let Some(filter) = server_filter {
                        if !name.starts_with(filter) {
                            continue;
                        }
                    }

                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let modified = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok();

                    backups.push((name.to_string(), size, modified));
                }
            }
        }
    }

    if backups.is_empty() {
        println!("{}", "No backups found.".yellow());
        return Ok(());
    }

    // Sort newest first
    backups.sort_by(|a, b| b.2.cmp(&a.2));

    println!(
        "{}",
        format!("Available Backups in {} ({} total):\n", backup_dir.display(), backups.len())
            .bold()
            .white()
    );

    println!(
        "  {:<42}  {:<12}",
        "BACKUP FILE".bold(),
        "SIZE".bold()
    );
    println!("  {}", "─".repeat(56).dimmed());

    for (name, size, _) in backups {
        let size_str = format_bytes(size);
        println!(
            "  {:<42}  {:<12}",
            name.cyan(),
            size_str.yellow()
        );
    }
    println!("  {}", "─".repeat(56).dimmed());
    println!();

    Ok(())
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
