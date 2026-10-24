use std::process::Command;

pub struct CreateServerOptions<'a> {
    pub server_name: &'a str,
    pub port: u16,
    pub server_type: &'a str,
    pub memory: &'a str,
    pub online_mode: bool,
}

/// Verifies that the Docker CLI is installed and the daemon is accessible.
pub fn ensure_docker_available() -> Result<(), String> {
    match Command::new("docker").arg("info").output() {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                Err(format!(
                    "Docker daemon is not reachable. Is Docker running?\nDetails: {}",
                    err_msg.trim()
                ))
            }
        }
        Err(e) => Err(format!(
            "Failed to execute 'docker' command. Ensure Docker is installed and in your PATH.\nError: {}",
            e
        )),
    }
}

/// Checks whether a container with the given name exists (running or stopped).
pub fn container_exists(name: &str) -> Result<bool, String> {
    let filter_arg = format!("^/?{}$", name);
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={}", filter_arg),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|e| format!("Failed to inspect docker containers: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let exists = stdout
        .lines()
        .any(|line| line.trim() == name || line.trim() == format!("/{}", name));

    Ok(exists)
}

/// Runs a new itzg/minecraft-server container with the requested configuration.
pub fn run_minecraft_container(opts: &CreateServerOptions) -> Result<String, String> {
    let volume_name = format!("golem_{}_data", opts.server_name);
    let port_mapping = format!("{}:25565", opts.port);
    let online_mode_val = if opts.online_mode { "TRUE" } else { "FALSE" };

    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            opts.server_name,
            "-p",
            &port_mapping,
            "-e",
            "EULA=TRUE",
            "-e",
            &format!("TYPE={}", opts.server_type),
            "-e",
            &format!("MEMORY={}", opts.memory),
            "-e",
            &format!("ONLINE_MODE={}", online_mode_val),
            "-v",
            &format!("{}:/data", volume_name),
            "--restart",
            "unless-stopped",
            "itzg/minecraft-server:latest",
        ])
        .output()
        .map_err(|e| format!("Failed to execute 'docker run': {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Docker error: {}", err.trim()));
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(container_id)
}

/// Starts an existing stopped container.
pub fn start_container(name: &str) -> Result<(), String> {
    let output = Command::new("docker")
        .args(["start", name])
        .output()
        .map_err(|e| format!("Failed to execute docker start: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Docker start error: {}", err.trim()));
    }

    Ok(())
}

/// Stops a running container.
pub fn stop_container(name: &str) -> Result<(), String> {
    let output = Command::new("docker")
        .args(["stop", name])
        .output()
        .map_err(|e| format!("Failed to execute docker stop: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Docker stop error: {}", err.trim()));
    }

    Ok(())
}

/// Gets the status of the container (e.g. "running", "exited", "paused").
pub fn get_container_status(name: &str) -> Result<String, String> {
    let output = Command::new("docker")
        .args(["inspect", "--format={{.State.Status}}", name])
        .output()
        .map_err(|e| format!("Failed to inspect container status: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(err.trim().to_string());
    }

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(status)
}

/// Gets the host port mapped to Minecraft's 25565 port.
pub fn get_container_port(name: &str) -> Option<u16> {
    // Check HostConfig PortBindings first (works even when container is stopped)
    let output = Command::new("docker")
        .args([
            "inspect",
            "--format={{range $p, $conf := .HostConfig.PortBindings}}{{range $conf}}{{.HostPort}} {{end}}{{end}}",
            name,
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        for item in text.split_whitespace() {
            if let Ok(p) = item.parse::<u16>() {
                return Some(p);
            }
        }
    }

    // Fallback to docker port command
    let port_output = Command::new("docker")
        .args(["port", name, "25565"])
        .output()
        .ok()?;

    if port_output.status.success() {
        let text = String::from_utf8_lossy(&port_output.stdout);
        for line in text.lines() {
            if let Some(pos) = line.rfind(':') {
                if let Ok(p) = line[pos + 1..].trim().parse::<u16>() {
                    return Some(p);
                }
            }
        }
    }

    None
}

/// Retrieves CPU and Memory statistics for a running container.
pub fn get_container_stats(name: &str) -> Result<(String, String), String> {
    let output = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.CPUPerc}}\t{{.MemUsage}}",
            name,
        ])
        .output()
        .map_err(|e| format!("Failed to run docker stats: {}", e))?;

    if !output.status.success() {
        return Err("Unavailable (Container stopped)".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    let parts: Vec<&str> = line.split('\t').collect();

    if parts.len() >= 2 {
        Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        Ok(("0.00%".to_string(), "N/A".to_string()))
    }
}

/// Fetches the last `tail` lines of container logs.
pub fn get_container_logs(name: &str, tail: usize) -> Result<Vec<String>, String> {
    let tail_str = tail.to_string();
    let output = Command::new("docker")
        .args(["logs", "--tail", &tail_str, name])
        .output()
        .map_err(|e| format!("Failed to get logs: {}", e))?;

    // Combine stdout and stderr because Minecraft server logs can be on either
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut logs: Vec<String> = stdout
        .lines()
        .chain(stderr.lines())
        .map(|s| s.to_string())
        .collect();

    // If both had output, keep the last `tail`
    if logs.len() > tail {
        logs = logs.split_off(logs.len() - tail);
    }

    Ok(logs)
}
