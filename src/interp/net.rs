//! The TCP sockets behind `java.net.ServerSocket` and `Socket` in `std/javalib/socket.scala`, blocking
//! and on the program's one thread: a server accepts one client at a time, its timeout
//! (`setSoTimeout`) bounding each accept; a socket's two streams are streams of `process.rs`, its
//! timeout bounding each read, its writes made in parts. A wait (an accept, a connect on a thread
//! of its own, a read, a write the peer does not take) sees a signal between slices, as the other
//! blocking natives.

use super::builtins::{reg, Table};
use super::value::*;
use super::*;
use std::time::{Duration, Instant};

type A<'a> = &'a [Value];

thread_local! {
    static SERVERS: RefCell<Vec<Option<std::net::TcpListener>>> = const { RefCell::new(Vec::new()) };
}

fn int(it: &mut Interp, a: A, i: usize) -> R<i32> {
    match a.get(i).and_then(|v| v.as_i32()) {
        Some(v) => Ok(v),
        None => it.unsupported(format!("argument {} is no Int", i)),
    }
}

/// The JDK's exception for a failed bind or connect: `BindException` for an address in use,
/// `ConnectException` for a refusal, `UnknownHostException` for a name that resolves to nothing,
/// else an `IOException`, each with the system's reason.
fn failed<T>(it: &mut Interp, e: &std::io::Error, host: &str) -> R<T> {
    let text = e.to_string();
    let reason = match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_string(),
        _ => text,
    };
    let class = match e.kind() {
        std::io::ErrorKind::AddrInUse | std::io::ErrorKind::AddrNotAvailable => "BindException",
        std::io::ErrorKind::ConnectionRefused => "ConnectException",
        std::io::ErrorKind::TimedOut => "SocketTimeoutException",
        _ if e.raw_os_error().is_none() => return it.throw_new(&["java", "net", "UnknownHostException"], vec![Value::str(host)]),
        _ => return super::process::io_error(it, &reason),
    };
    it.throw_new(&["java", "net", class], vec![Value::string(reason)])
}

/// A connected socket's streams and ends: its input stream, its output stream (each a stream of
/// `process.rs`), the peer's port and the local one.
fn connected(it: &mut Interp, sock: std::net::TcpStream) -> R {
    let _ = sock.set_nonblocking(false);
    let (peer, local) = (sock.peer_addr().map_or(0, |a| a.port()), sock.local_addr().map_or(0, |a| a.port()));
    let host = sock.peer_addr().map_or(String::new(), |a| a.ip().to_string());
    let out = match sock.try_clone() {
        Ok(out) => out,
        Err(e) => return failed(it, &e, ""),
    };
    let input = super::process::socket_stream(sock);
    let output = super::process::socket_stream(out);
    Ok(Value::array(vec![Value::Long(input as i64), Value::Long(output as i64), Value::Long(peer as i64), Value::Long(local as i64), Value::string(host)]))
}

pub(super) fn install(it: &mut Table) {
    // `new ServerSocket(port, backlog, address)`: bound to the address, or every address when it
    // is null (both IPv6 and IPv4 where the system allows, as the JDK binds), port 0 any free one.
    reg!(it, "java.net.serverOpen", |it, a| {
        if it.pure {
            return it.impure("a socket");
        }
        let host = match a.first() {
            Some(Value::Str(h)) => Some(h.to_string()),
            _ => None,
        };
        let port = int(it, a, 1)?;
        if !(0..=65535).contains(&port) {
            return it.throw_named("IllegalArgumentException", &format!("Port value out of range: {}", port));
        }
        let bound = match &host {
            Some(h) => std::net::TcpListener::bind((h.as_str(), port as u16)),
            None => std::net::TcpListener::bind(("::", port as u16)).or_else(|_| std::net::TcpListener::bind(("0.0.0.0", port as u16))),
        };
        match bound {
            Ok(listener) => Ok(Value::Int(SERVERS.with(|s| {
                let mut servers = s.borrow_mut();
                servers.push(Some(listener));
                (servers.len() - 1) as i32
            }))),
            Err(e) => failed(it, &e, host.as_deref().unwrap_or("")),
        }
    });
    reg!(it, "java.net.serverPort", |it, a| {
        let id = int(it, a, 0)?;
        let port = SERVERS.with(|s| s.borrow().get(id as usize).and_then(|l| l.as_ref()).and_then(|l| l.local_addr().ok()).map_or(-1, |a| a.port() as i32));
        Ok(Value::Int(port))
    });
    // `accept`: the next client, waiting at most `timeout` milliseconds (none for 0), else the
    // JDK's `SocketTimeoutException`.
    reg!(it, "java.net.serverAccept", |it, a| {
        let id = int(it, a, 0)?;
        let timeout = int(it, a, 1)?;
        if it.pure {
            return it.impure("a socket");
        }
        it.flush();
        let end = (timeout > 0).then(|| Instant::now() + Duration::from_millis(timeout as u64));
        loop {
            let accepted = SERVERS.with(|s| {
                let servers = s.borrow();
                let l = servers.get(id as usize).and_then(|l| l.as_ref())?;
                let _ = l.set_nonblocking(true);
                Some(l.accept())
            });
            match accepted {
                None => return it.throw_new(&["java", "net", "SocketException"], vec![Value::str("Socket is closed")]),
                Some(Ok((sock, _))) => return connected(it, sock),
                Some(Err(e)) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::Interrupted => {}
                Some(Err(e)) => return failed(it, &e, ""),
            }
            if end.is_some_and(|e| Instant::now() >= e) {
                return it.throw_new(&["java", "net", "SocketTimeoutException"], vec![Value::str("Accept timed out")]);
            }
            std::thread::sleep(Duration::from_millis(2));
            super::process::signal_seen(it)?;
        }
    });
    reg!(it, "java.net.serverClose", |it, a| {
        let id = int(it, a, 0)?;
        SERVERS.with(|s| {
            if let Some(slot) = s.borrow_mut().get_mut(id as usize) {
                *slot = None;
            }
        });
        Ok(Value::Unit)
    });
    // `new Socket(host, port)`, or `connect` with a timeout in milliseconds (none for 0).
    reg!(it, "java.net.socketConnect", |it, a| {
        if it.pure {
            return it.impure("a socket");
        }
        let host = it.str_arg(a, 0)?;
        let port = int(it, a, 1)?;
        let timeout = int(it, a, 2)?;
        use std::net::ToSocketAddrs;
        let addrs: Vec<std::net::SocketAddr> = match (host.as_ref(), port as u16).to_socket_addrs() {
            Ok(addrs) => addrs.collect(),
            Err(_) => return it.throw_new(&["java", "net", "UnknownHostException"], vec![Value::str(&host)]),
        };
        it.flush();
        let mut last = None;
        for addr in addrs {
            // The connection made on a thread of its own and waited for in slices, so that a signal
            // is seen meanwhile, as by every blocking native; a thread left behind ends with its
            // attempt.
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new().name("teq-connect".to_string()).stack_size(64 * 1024).spawn(move || {
                crate::alloc::enter();
                let r = if timeout > 0 { std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(timeout as u64)) } else { std::net::TcpStream::connect(addr) };
                let _ = tx.send(r);
            });
            if let Err(e) = spawned {
                return it.unsupported(format!("no thread to connect a socket: {}", e));
            }
            let r = loop {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(r) => break r,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => super::process::signal_seen(it)?,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break Err(std::io::Error::other("the connection's thread ended")),
                }
            };
            match r {
                Ok(sock) => return connected(it, sock),
                Err(e) => last = Some(e),
            }
        }
        match last {
            Some(e) => failed(it, &e, &host),
            None => it.throw_new(&["java", "net", "UnknownHostException"], vec![Value::str(&host)]),
        }
    });
    reg!(it, "java.net.socketShutdown", |it, a| {
        let id = int(it, a, 0)?;
        super::process::shutdown_stream(id, matches!(a.get(1), Some(Value::Bool(true))));
        Ok(Value::Unit)
    });
    // The numeric address a host name resolves to first, as `InetAddress.getByName` takes it.
    reg!(it, "java.net.resolveHost", |it, a| {
        let host = it.str_arg(a, 0)?;
        if it.pure {
            return it.impure("resolving a host");
        }
        use std::net::ToSocketAddrs;
        // The resolver's own reason, as the JDK gives it after the name.
        match (host.as_ref(), 0u16).to_socket_addrs().map(|mut i| i.next()) {
            Ok(Some(addr)) => Ok(Value::string(addr.ip().to_string())),
            Ok(None) => it.throw_new(&["java", "net", "UnknownHostException"], vec![Value::str(&host)]),
            Err(e) => {
                let text = e.to_string();
                let reason = text.strip_prefix("failed to lookup address information: ").unwrap_or(&text).to_string();
                it.throw_new(&["java", "net", "UnknownHostException"], vec![Value::string(format!("{}: {}", host, reason))])
            }
        }
    });
    reg!(it, "java.net.socketTimeout", |it, a| {
        let id = int(it, a, 0)?;
        let millis = int(it, a, 1)?;
        super::process::set_stream_timeout(id, millis);
        Ok(Value::Unit)
    });
}
