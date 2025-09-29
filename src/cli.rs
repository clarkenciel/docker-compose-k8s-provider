use crate::{cmd, docker};
use clap::{Args, Parser, Subcommand};

pub(crate) fn run(opts: Opts) {
    let Compose::Compose { project, command } = opts.compose;
    docker::info!("Running for {} {:?}", project, command);
    match command {
        ComposeCommand::Up(UpArgs {
            resource,
            port_mapping,
            service: ServiceArgs { service },
        }) => cmd::up(cmd::up::Args {
            port_mapping,
            resource,
            service,
            project,
        }),
        ComposeCommand::Down(UpArgs {
            service: ServiceArgs { service },
            ..
        }) => cmd::down(cmd::down::Args { project, service }),
    }
}

#[derive(Parser, Debug)]
pub(crate) struct Opts {
    #[command(subcommand)]
    pub(crate) compose: Compose,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Compose {
    Compose {
        #[arg(long = "project-name", short, alias = "project")]
        project: String,

        #[command(subcommand)]
        command: ComposeCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ComposeCommand {
    Up(UpArgs),

    Down(UpArgs),
}

#[derive(Args, Debug)]
pub(crate) struct ServiceArgs {
    /// Name of the service being managed by docker compose
    /// and provided by this provider.
    ///
    /// NB: This refers to the docker compose service, not a kubernetes resource.
    service: String,
}

#[derive(Args, Debug)]
pub(crate) struct UpArgs {
    /// Name of a resouce on the remote kubernetes cluster.
    ///
    /// e.g. `svc/some-service`, `deployment/some-deployment`, `pod/some-pod`
    #[arg(long, short)]
    pub(crate) resource: String,

    #[arg(long, short)]
    pub(crate) port_mapping: String,

    #[command(flatten)]
    pub(crate) service: ServiceArgs,
}
