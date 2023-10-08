use clap::Parser;
#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use std::{os::unix::prelude::PermissionsExt, path::PathBuf};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_subscriber::EnvFilter;
use webdriver::command::{PrintParameters, WebDriverCommand};

pub mod browser_client;
pub mod geckodriver;

pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms as _)).await
}

/// converts a file to pdf using browser's built-in print feature
/// currently supports only firefox + geckodriver (contributions are welcome)
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// output file (if this option is not present it defaults to STDOUT)
    #[arg(short, long, value_name = "FILE")]
    out_file: Option<PathBuf>,
    /// source file that will be loaded into the browser
    #[arg(short, long, value_name = "FILE")]
    source_file: PathBuf,
}

fn setup_logging() {
    use tracing_subscriber::prelude::*;
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_from("info").unwrap()))
        .with(tracing_subscriber::fmt::Layer::new().with_writer(std::io::stderr));
    if let Err(message) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("logging setup failed: {message:?}");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install().ok();
    setup_logging();
    tracing::debug!("{} v{}", clap::crate_name!(), clap::crate_version!());
    let Cli {
        source_file,
        out_file,
    } = Cli::parse();
    tracing::debug!(?source_file, "printing file");
    let geckodriver_path = PathBuf::from("/tmp/geckodriver");
    tokio::fs::write(&geckodriver_path, geckodriver::GECKODRIVER_BIN)
        .await
        .context("writing geckodriver")?; // TODO: better path
    tokio::fs::set_permissions(&geckodriver_path, std::fs::Permissions::from_mode(0o744))
        .await
        .wrap_err("setting permissions for geckodriver")?;
    let geckodriver_options = geckodriver::GeckodriverSpawnOptions {
        path: geckodriver_path,
        headless: true,
    };
    tracing::debug!("spawning a geckodriver + firefox instance");
    let firefox = browser_client::FirefoxClient::spawn(&geckodriver_options, 6689)
        .await
        .wrap_err("spawning firefox in headless mode")?;
    let file_uri = format!("file://{}", source_file.display());
    tracing::debug!(%file_uri, "navigating");
    firefox
        .firefox
        .client
        .goto(&file_uri)
        .await
        .wrap_err("opening file in firefox")?;
    tracing::debug!("issuing print command");
    let response = firefox
        .firefox
        .client
        .issue_cmd(WebDriverCommand::Print(PrintParameters {
            // orientation: todo!(),
            // scale: todo!(),
            // background: todo!(),
            // page: todo!(),
            // margin: todo!(),
            // page_ranges: todo!(),
            // shrink_to_fit: todo!(),
            ..PrintParameters::default()
        }))
        .await
        .wrap_err("printing page")?;
    tracing::debug!(%response, "received a response");
    match response {
        serde_json::Value::String(contents) => {
            let mut contents = std::io::Cursor::new(contents.as_bytes());
            let mut stdout = std::io::stdout().lock();
            let engine = base64::engine::GeneralPurpose::new(
                &base64::alphabet::STANDARD,
                base64::engine::general_purpose::PAD,
            );
            let mut decoder = base64::read::DecoderReader::new(&mut contents, &engine);
            match out_file {
                Some(out_file) => {
                    {
                        let mut out_file = std::fs::File::create(&out_file)
                            .wrap_err("opening output file for writing")?;
                        std::io::copy(&mut decoder, &mut out_file)
                            .context("decoding base64 output and writing to a file")?;
                    }
                    println!("{}", out_file.display());
                }
                None => {
                    std::io::copy(&mut decoder, &mut stdout)
                        .context("decoding base64 output and writing to stdout")?;
                }
            }
        }
        other => {
            eyre::bail!(
                "unexpected value returned by webdriver (expected Value::String, got {other:#?})"
            );
        }
    };

    Ok(())
}
