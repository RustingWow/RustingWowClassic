use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use extract_dbc::extract_dbcs;
use wow_shared::init_tracing;

fn main() -> ExitCode {
    init_tracing();
    match run() {
        Ok(count) => {
            tracing::info!(count, "extracted DBC files");
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<usize> {
    let args = Args::parse()?;
    extract_dbcs(&args.client, &args.out, args.locale.as_deref())
}

struct Args {
    client: PathBuf,
    out: PathBuf,
    locale: Option<String>,
}

impl Args {
    fn parse() -> anyhow::Result<Self> {
        let mut client = env::var("WOW_CLIENT_PATH").ok().map(PathBuf::from);
        let mut out = env::var("DBC_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data/dbc"));
        let mut locale = None;
        let mut argv = env::args().skip(1);
        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "--client" => {
                    client = Some(required_value("--client", argv.next())?.into());
                }
                "--out" => {
                    out = required_value("--out", argv.next())?.into();
                }
                "--locale" => {
                    locale = Some(required_value("--locale", argv.next())?);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => anyhow::bail!("unknown argument {other} (see --help)"),
            }
        }
        let Some(client) = client else {
            print_usage();
            anyhow::bail!("--client or WOW_CLIENT_PATH is required");
        };
        Ok(Self {
            client,
            out,
            locale,
        })
    }
}

fn required_value(flag: &str, value: Option<String>) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
}

fn print_usage() {
    eprintln!(
        "\
extract-dbc — copy Vanilla 1.12.1 DBFilesClient/*.dbc out of the client MPQs

Usage:
  cargo run -p extract-dbc -- --client /path/to/WoW [--out data/dbc] [--locale enUS]

Environment:
  WOW_CLIENT_PATH   default for --client
  DBC_DIR           default for --out (data/dbc)
"
    );
}
