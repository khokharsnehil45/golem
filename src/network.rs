use std::net::{TcpListener, UdpSocket};

/// Attempts to retrieve the machine's primary local LAN IP address.
pub fn get_local_ip() -> Option<String> {
    // Connect to a public DNS IP over UDP. No actual network packet is sent,
    // but the OS kernel chooses the outgoing interface and assigns its local IP.
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    let ip = local_addr.ip();
    if ip.is_loopback() || ip.is_unspecified() {
        None
    } else {
        Some(ip.to_string())
    }
}

/// Checks if a TCP port is currently available on 0.0.0.0
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}
