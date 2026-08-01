use market_time_data::load_ruleset;
use market_time_server::app;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

const USAGE: &str = "\
market-time-server --dataset <path> [--bind <address>]

Defaults:
  --bind 127.0.0.1:8080
";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("market-time-server: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }

    init_tracing();
    let dataset = options
        .dataset
        .ok_or_else(|| format!("--dataset <path> is required\n\n{USAGE}"))?;
    let ruleset = load_ruleset(&dataset).map_err(|error| error.to_string())?;
    let listener = tokio::net::TcpListener::bind(&options.bind)
        .await
        .map_err(|error| error.to_string())?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    tracing::info!(%address, "market-time-server listening");

    axum::serve(listener, app(ruleset))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| error.to_string())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("market_time_server=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "failed to listen for Ctrl-C");
            }
        }
        () = terminate_signal() => {}
    }
    tracing::info!("shutdown signal received");
}

#[cfg(unix)]
async fn terminate_signal() {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut signal) => {
            signal.recv().await;
        }
        Err(error) => {
            tracing::error!(%error, "failed to listen for SIGTERM");
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(not(unix))]
async fn terminate_signal() {
    std::future::pending::<()>().await;
}

struct Options {
    dataset: Option<PathBuf>,
    bind: String,
    help: bool,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            dataset: None,
            bind: "127.0.0.1:8080".to_owned(),
            help: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => options.help = true,
                "--dataset" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--dataset requires a path".to_owned())?;
                    options.dataset = Some(PathBuf::from(value));
                }
                "--bind" => {
                    options.bind = args
                        .next()
                        .ok_or_else(|| "--bind requires an address".to_owned())?;
                }
                other => return Err(format!("unknown argument {other:?}\n\n{USAGE}")),
            }
        }
        Ok(options)
    }
}
