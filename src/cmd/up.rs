use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{Receiver, channel};

use serde::Serialize;

use nix::unistd::{self, ForkResult, Pid, setsid};

use crate::net::socket_fn;
use crate::result::Result;
use crate::{docker, net, protocol};

#[derive(Debug, Serialize)]
pub(crate) struct Args {
    pub(crate) project: String,
    pub(crate) service: String,
    pub(crate) port_mapping: String,
    pub(crate) resource: String,
}

pub(crate) fn up(args: Args) {
    match spawn_daemon() {
        Ok(ForkResult::Child) => match run_daemon(args) {
            Err(e) => {
                docker::error!("Daemon failed to start: {}", e);
                process::exit(1);
            }
            _ => {
                process::exit(0);
            }
        },
        Ok(ForkResult::Parent { child }) => {
            docker::info!("Spawning process started at: {}", child);
            match health_checks(args, child) {
                Err(e) => {
                    docker::error!("{}", e);
                    process::exit(1)
                }
                Ok(()) => {
                    docker::info!("Daemon up");
                    process::exit(0);
                }
            };
        }
        Err(e) => {
            docker::error!("{}", e);
            process::exit(1);
        }
    }
}

/// Spawn a daemon process using double-fork and setsid
/// The `ForkResult::Child` that is returned will be the
/// `ForkResult` for the daemon process, i.e. the grandchild of
/// the original process started started by `docker compose`.
///
/// If a parent result is returned the caller can use the provided
/// child Pid to check whether the process exits unsuccessfully.
fn spawn_daemon() -> Result<ForkResult> {
    match fork()? {
        fr @ ForkResult::Parent { .. } => Ok(fr),
        ForkResult::Child => {
            setsid()?;

            docker::info!("New session started, forking daemon");
            match fork() {
                Ok(child @ ForkResult::Child) => {
                    daemonize()?;
                    Ok(child)
                }
                Ok(ForkResult::Parent { child }) => {
                    docker::info!("Child process forked at {}. exiting", child);
                    std::process::exit(0)
                }
                Err(e) => {
                    docker::error!("Failed to spawn child: {}", e);
                    std::process::exit(-1)
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn daemonize() -> Result<()> {
    Ok(unistd::daemon(false, false)?)
}

#[cfg(target_os = "macos")]
fn daemonize() -> Result<()> {
    #[allow(deprecated)]
    match unsafe { libc::daemon(0, 0) } {
        0 => Ok(()),
        _ => Err(nix::errno::Errno::last().into()),
    }
}

fn run_daemon(args: Args) -> Result<()> {
    std::thread::scope(|scope| {
        let (send, recv) = channel();
        scope.spawn(|| manage_kubectl(&args, recv));

        let result = handle_health_requests(&args);
        send.send(())?;
        result
    })?;

    Ok(())
}

fn handle_health_requests(args: &Args) -> Result<()> {
    let socket = socket_fn(&args.project, &args.service);

    let server = net::Server::listen(&socket)?;
    docker::info!("Listening on {}", socket.display());
    for mut connection in server.incoming().filter_map(std::result::Result::ok) {
        let Ok(mut writer) = connection.try_clone() else {
            continue;
        };

        loop {
            match protocol::Request::from_reader(&mut connection) {
                Err(_e) => break,
                Ok(protocol::Request::Health) => {
                    writer.write_all(&protocol::Response::Ok.as_bytes())?
                }
                Ok(protocol::Request::Stop) => return Ok(()),
            };
        }
    }

    Ok(())
}

struct PortForward(process::Child);

impl Drop for PortForward {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn manage_kubectl(args: &Args, interrupt: Receiver<()>) -> Result<()> {
    let mut command = process::Command::new("kubectl");
    command.args(["port-forward", &args.resource, &args.port_mapping]);

    loop {
        let mut child = PortForward(command.spawn()?);

        loop {
            if let Ok(()) = interrupt.try_recv() {
                return Ok(());
            }

            if let Err(e) = child.0.try_wait() {
                tracing::warn!("Kubectl command failed: {}", e)
            };
        }
    }
}

fn health_checks(args: Args, pid: Pid) -> Result<()> {
    nix::sys::wait::waitpid(pid, None)?;
    let socket = socket_fn(&args.project, &args.service);
    let mut client = net::connect_client(&socket)?;
    match client.request(protocol::Request::Health)? {
        protocol::Response::Err => Err(anyhow::format_err!("Health check failed")),
        protocol::Response::Ok => Ok(()),
    }
}

/// Fork is primarily unsafe if used in a multithreaded program
/// and with a child process that uses certain functions that are not
/// safe in async contexts.
///
/// This program is not multithreaded so we don't have to worry.
fn fork() -> Result<ForkResult> {
    Ok(unsafe { unistd::fork() }?)
}
