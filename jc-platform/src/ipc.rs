use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{fs, io, thread};

#[derive(Serialize, Deserialize)]
struct IpcMessage {
    command: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct IpcResponse {
    ok: bool,
}

/// Returns the IPC endpoint path.
///
/// - Unix: `~/.config/jc/jc.sock` (Unix domain socket)
/// - Windows: uses TCP on localhost (socket file not supported)
fn socket_path() -> PathBuf {
    let config_dir =
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()));
    config_dir.join("jc").join("jc.sock")
}

/// Try to send an `open_project` command to an already-running instance.
/// Returns `true` if the message was delivered successfully.
pub fn try_send_to_running(path: &Path) -> bool {
    #[cfg(unix)]
    {
        unix_try_send(path)
    }
    #[cfg(not(unix))]
    {
        tcp_try_send(path)
    }
}

/// Remove the socket/endpoint. Safe to call during cleanup.
pub fn cleanup_socket() {
    let _ = fs::remove_file(socket_path());
}

pub struct SocketServer {
    shutdown: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    #[allow(dead_code)]
    endpoint: Endpoint,
}

#[allow(dead_code)]
enum Endpoint {
    Unix(PathBuf),
    Tcp(std::net::SocketAddr),
}

impl SocketServer {
    pub fn bind(callback: impl Fn(PathBuf) + Send + 'static) -> io::Result<SocketServer> {
        #[cfg(unix)]
        {
            Self::bind_unix(callback)
        }
        #[cfg(not(unix))]
        {
            Self::bind_tcp(callback)
        }
    }

    #[cfg(unix)]
    fn bind_unix(callback: impl Fn(PathBuf) + Send + 'static) -> io::Result<SocketServer> {
        use std::os::unix::net::UnixListener;

        let path = socket_path();

        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Remove stale socket if present.
        let _ = fs::remove_file(&path);

        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);

        let thread = thread::spawn(move || {
            while !shutdown_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                        handle_connection(stream, &callback);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        });

        Ok(SocketServer {
            shutdown,
            thread: Some(thread),
            endpoint: Endpoint::Unix(path),
        })
    }

    #[cfg(not(unix))]
    fn bind_tcp(callback: impl Fn(PathBuf) + Send + 'static) -> io::Result<SocketServer> {
        use std::net::TcpListener;

        // Use a fixed port for IPC. Write the port to the socket_path file so
        // clients can discover it.
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        listener.set_nonblocking(true)?;

        // Write port file so clients can discover us
        let port_file = socket_path();
        if let Some(parent) = port_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&port_file, addr.port().to_string())?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);

        let thread = thread::spawn(move || {
            while !shutdown_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                        handle_connection(stream, &callback);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        });

        Ok(SocketServer {
            shutdown,
            thread: Some(thread),
            endpoint: Endpoint::Tcp(addr),
        })
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Briefly connect to unblock the accept loop.
        match &self.endpoint {
            #[cfg(unix)]
            Endpoint::Unix(path) => {
                let _ = std::os::unix::net::UnixStream::connect(path);
            }
            #[cfg(not(unix))]
            Endpoint::Tcp(addr) => {
                let _ = std::net::TcpStream::connect(addr);
            }
            _ => {}
        }

        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }

        // Clean up endpoint file
        let _ = fs::remove_file(socket_path());
    }
}

// --- Platform-specific client implementations ---

#[cfg(unix)]
fn unix_try_send(path: &Path) -> bool {
    use std::os::unix::net::UnixStream;

    let sock = socket_path();
    let stream = match UnixStream::connect(&sock) {
        Ok(s) => s,
        Err(_) => {
            let _ = fs::remove_file(&sock);
            return false;
        }
    };

    send_over_stream(stream, path)
}

#[cfg(not(unix))]
fn tcp_try_send(path: &Path) -> bool {
    use std::net::TcpStream;

    let port_file = socket_path();
    let port_str = match fs::read_to_string(&port_file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let port: u16 = match port_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let stream = match TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(_) => {
            let _ = fs::remove_file(&port_file);
            return false;
        }
    };

    send_over_stream(stream, path)
}

fn send_over_stream<S: io::Read + io::Write>(mut stream: S, path: &Path) -> bool {
    let msg = IpcMessage {
        command: "open_project".into(),
        path: path.to_string_lossy().into_owned(),
    };

    let mut line = match serde_json::to_string(&msg) {
        Ok(s) => s,
        Err(_) => return false,
    };
    line.push('\n');

    if stream.write_all(line.as_bytes()).is_err() || stream.flush().is_err() {
        return false;
    }

    let mut reader = BufReader::new(&mut stream);
    let mut resp_line = String::default();
    if reader.read_line(&mut resp_line).is_err() {
        return false;
    }

    serde_json::from_str::<IpcResponse>(&resp_line)
        .map(|r| r.ok)
        .unwrap_or(false)
}

fn handle_connection<S: io::Read + io::Write>(mut stream: S, callback: &impl Fn(PathBuf)) {
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::default();
    if reader.read_line(&mut line).is_err() {
        return;
    }

    let msg: IpcMessage = match serde_json::from_str(&line) {
        Ok(m) => m,
        Err(_) => return,
    };

    if msg.command == "open_project" {
        callback(PathBuf::from(msg.path));
    }

    // Need to drop reader to regain access to stream
    drop(reader);

    let resp = IpcResponse { ok: true };
    if let Ok(mut resp_json) = serde_json::to_string(&resp) {
        resp_json.push('\n');
        let _ = stream.write_all(resp_json.as_bytes());
        let _ = stream.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn socket_path_is_under_config_dir() {
        let path = socket_path();
        assert!(path.to_string_lossy().contains("jc"));
        assert!(path.to_string_lossy().ends_with("jc.sock"));
    }

    #[test]
    fn ipc_roundtrip() {
        let (tx, rx) = flume::unbounded();
        let server = SocketServer::bind(move |path| {
            let _ = tx.send(path);
        });
        assert!(server.is_ok());

        let test_path = PathBuf::from("/tmp/test-project");
        let sent = try_send_to_running(&test_path);
        assert!(sent);

        let received = rx.recv_timeout(Duration::from_secs(2));
        assert!(received.is_ok());
        assert_eq!(received.unwrap(), test_path);
    }
}
