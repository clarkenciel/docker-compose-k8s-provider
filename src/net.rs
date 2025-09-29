use std::{
    fs,
    io::{ErrorKind, Write as _},
    os::unix::net::{Incoming, UnixListener, UnixStream},
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{docker, protocol, result::Result};

pub(crate) struct Server {
    socket_path: PathBuf,
    listener: UnixListener,
}

impl Server {
    pub(crate) fn listen<S: AsRef<Path>>(socket: S) -> Result<Self> {
        if another_server_listening(&socket) {
            return Err(ServerError::AddrInUse(socket.as_ref().display().to_string()).into());
        }

        let parent = socket.as_ref().parent().unwrap_or(socket.as_ref());
        std::fs::create_dir_all(parent).map_err(ServerError::ListenError)?;

        let listener = UnixListener::bind(&socket).map_err(ServerError::ListenError)?;
        listener
            .set_nonblocking(true)
            .map_err(ServerError::NonBlockingUnavailable)?;
        let socket_path = socket.as_ref().to_path_buf();
        Ok(Self {
            listener,
            socket_path,
        })
    }

    pub(crate) fn incoming(&self) -> Incoming<'_> {
        self.listener.incoming()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

#[derive(Error, Debug)]
pub(crate) enum ServerError {
    #[error("Another server is already bound to this socket {0}")]
    AddrInUse(String),

    #[error("Failed to establish listener: {0}")]
    ListenError(std::io::Error),

    #[error("failed to set server as non-blocking")]
    NonBlockingUnavailable(std::io::Error),
}

fn another_server_listening<P: AsRef<Path>>(socket: P) -> bool {
    if !socket.as_ref().exists() {
        return false;
    }

    match UnixStream::connect(socket) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub(crate) struct Client {
    inner: UnixStream,
}

impl Client {
    pub(crate) fn send(&mut self, message: protocol::Request) -> Result<()> {
        self.inner.write_all(&message.as_bytes())?;
        Ok(())
    }

    pub(crate) fn receive(&mut self) -> Result<protocol::Response> {
        protocol::Response::from_reader(&mut self.inner).map_err(|e| e.into())
    }

    pub(crate) fn request(&mut self, message: protocol::Request) -> Result<protocol::Response> {
        self.send(message)?;
        self.receive()
    }

    pub(crate) fn wait_for_disconnect(&self) -> Result<()> {
        let addr = self.inner.peer_addr()?;
        for _ in 0..10 {
            match UnixStream::connect_addr(&addr) {
                Err(e)
                    if e.kind() == ErrorKind::AddrNotAvailable
                        || e.kind() == ErrorKind::NotFound =>
                {
                    return Ok(());
                }
                Err(e) => {
                    docker::error!("Unexpected error waiting for server disconnect: {}", e);
                }
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Err(anyhow::format_err!("Disconnect wait timed out"))
    }
}

pub(crate) fn connect_client<P: AsRef<Path>>(
    socket: P,
) -> std::result::Result<Client, ClientError> {
    for _ in 0..15 {
        match UnixStream::connect(&socket) {
            Err(e) => {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Ok(stream) => return Ok(Client { inner: stream }),
        }
    }

    Err(ClientError::ConnectTimeout)
}

#[derive(Error, Debug)]
pub(crate) enum ClientError {
    #[error("Connect timed out")]
    ConnectTimeout,
}

pub(crate) fn socket_fn(project: &str, service: &str) -> PathBuf {
    let mut buf = PathBuf::from("/tmp");
    buf.push(project);
    buf.push(service);
    buf.set_extension("sock");
    buf
}
