/// Validates that a server name complies with Docker container naming standards.
pub fn validate_server_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Server name cannot be empty.");
    }
    if name.len() > 63 {
        return Err("Server name cannot exceed 63 characters.");
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphanumeric() {
        return Err("Server name must begin with an alphanumeric character (a-z, A-Z, 0-9).");
    }

    let is_valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');

    if !is_valid {
        return Err("Server name can only contain alphanumeric characters, underscores, periods, and hyphens [a-zA-Z0-9_.-].");
    }

    Ok(())
}

/// Validates memory string format (e.g. 2G, 4G, 1024M).
pub fn validate_memory(mem: &str) -> Result<(), &'static str> {
    let mem = mem.trim();
    if mem.len() < 2 {
        return Err("Memory string is too short. Expected format like '2G' or '1024M'.");
    }

    let (amount, unit) = mem.split_at(mem.len() - 1);
    let valid_unit = matches!(unit, "G" | "g" | "M" | "m");
    let valid_number = amount.chars().all(|c| c.is_ascii_digit()) && !amount.is_empty();

    if !valid_unit || !valid_number {
        return Err("Invalid memory format. Use numbers followed by 'G' (Gigabytes) or 'M' (Megabytes), e.g., '2G' or '4096M'.");
    }

    Ok(())
}

/// Validates that the requested port is not 0.
pub fn validate_port(port: u16) -> Result<(), &'static str> {
    if port == 0 {
        return Err("Port 0 is reserved. Please specify a valid port between 1 and 65535.");
    }
    Ok(())
}

/// Validates maximum players limit.
pub fn validate_max_players(count: u32) -> Result<(), &'static str> {
    if count == 0 {
        return Err("Max players must be at least 1.");
    }
    if count > 10000 {
        return Err("Max players cannot exceed 10,000.");
    }
    Ok(())
}

/// Validates view distance (render distance in chunks).
pub fn validate_view_distance(distance: u8) -> Result<(), &'static str> {
    if !(3..=32).contains(&distance) {
        return Err("View distance must be between 3 and 32 chunks.");
    }
    Ok(())
}
