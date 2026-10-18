//! Which machines may talk to the remote control when it listens on a
//! network address, and which address to show the user.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

/// Loopback, private, link-local and shared (CGNAT, used by Tailscale and
/// the like) addresses. A network-facing listener refuses everything else,
/// so a computer with a public address, or a router forwarding the port,
/// does not put the remote control on the internet.
pub fn is_local_network(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || (a == 100 && (64..128).contains(&b))
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            v6.is_loopback()
                // fc00::/7, unique local addresses
                || (first & 0xfe00) == 0xfc00
                // fe80::/10, link-local
                || (first & 0xffc0) == 0xfe80
        }
    }
}

/// This computer's address on the local network, for the link and QR code
/// shown in the settings. Connecting a UDP socket sends nothing; it only
/// asks the system which interface would carry traffic to that address.
pub fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    // TEST-NET-1 (RFC 5737): never routed, so nothing is ever reached.
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_networks_only() {
        for ok in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.9",
            "192.168.1.20",
            "169.254.3.4",
            "100.101.102.103",
            "::1",
            "fd12:3456::1",
            "fe80::1",
            "::ffff:192.168.1.5",
        ] {
            assert!(is_local_network(ok.parse().unwrap()), "{ok}");
        }
        for refused in [
            "8.8.8.8",
            "172.32.0.1",
            "100.128.0.1",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
        ] {
            assert!(!is_local_network(refused.parse().unwrap()), "{refused}");
        }
    }
}
