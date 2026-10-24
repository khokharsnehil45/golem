use colored::*;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::docker;

pub fn display_logs(server_name: &str, follow: bool, tail: usize) -> Result<(), String> {
    // 1. Check if server container exists
    match docker::container_exists(server_name) {
        Ok(true) => {}
        Ok(false) => {
            return Err(format!(
                "Server '{}' does not exist. Use 'golem ls' to see available servers.",
                server_name
            ));
        }
        Err(e) => return Err(format!("Failed to inspect server: {}", e)),
    }

    if !follow {
        // Static fetch of recent logs
        let logs = docker::get_container_logs(server_name, tail)?;
        if logs.is_empty() {
            println!("{}", "No logs available yet for this container.".yellow());
            return Ok(());
        }

        for line in logs {
            print_colorized_log(&line);
        }
        return Ok(());
    }

    // Live stream logs with follow (-f)
    println!(
        "{} Streaming live logs for '{}' (Press {} to detach)\n",
        "•".cyan(),
        server_name.bold().cyan(),
        "Ctrl+C".bold().yellow()
    );

    let tail_str = tail.to_string();
    let mut child = Command::new("docker")
        .args(["logs", "-f", "--tail", &tail_str, server_name])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn docker logs: {}", e))?;

    // Combine stdout and stderr
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut threads = Vec::new();

    if let Some(out) = stdout {
        threads.push(std::thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().map_while(Result::ok) {
                print_colorized_log(&line);
            }
        }));
    }

    if let Some(err) = stderr {
        threads.push(std::thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().map_while(Result::ok) {
                print_colorized_log(&line);
            }
        }));
    }

    for t in threads {
        let _ = t.join();
    }

    let _ = child.wait();
    Ok(())
}

fn print_colorized_log(line: &str) {
    if line.contains("WARN") {
        println!("{}", line.yellow());
    } else if line.contains("ERROR") || line.contains("Exception") || line.contains("FATAL") {
        println!("{}", line.bold().red());
    } else if line.contains("joined the game") {
        println!("{}", line.bold().green());
    } else if line.contains("left the game") {
        println!("{}", line.bold().yellow());
    } else if line.contains("Done") {
        println!("{}", line.bold().cyan());
    } else {
        println!("{}", line.white());
    }
}
