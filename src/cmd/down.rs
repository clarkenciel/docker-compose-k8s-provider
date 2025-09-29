use std::process;

use crate::{docker, net, protocol};

pub(crate) struct Args {
    pub(crate) project: String,
    pub(crate) service: String,
}

pub(crate) fn down(args: Args) {
    let socket = net::socket_fn(&args.project, &args.service);
    docker::info!("Removing daemon at {}", socket.display());

    let mut client = match net::connect_client(socket) {
        Ok(c) => c,
        Err(e) => {
            docker::error!("Failed to connect to daemon: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = client.send(protocol::Request::Stop) {
        docker::error!("Failed to send daemon kill message: {}", e);
        process::exit(1);
    }

    if let Err(e) = client.wait_for_disconnect() {
        docker::error!("Failed to kill daemon: {}", e);
        process::exit(1);
    }
}
