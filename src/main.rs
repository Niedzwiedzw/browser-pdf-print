use clap::Parser;
#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use std::path::{Path, PathBuf};
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
    #[arg(short, long, value_name = "FILE", default_value = "geckodriver")]
    geckodriver_path: PathBuf,
    #[arg(short, long, value_name = "FILE")]
    /// saves output file at the same path as source, changing only the extension
    update_extension: bool,
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
        update_extension,
        geckodriver_path,
    } = Cli::parse();
    tracing::debug!(?source_file, "printing file");
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
            let mut save_to_file = |out_file: &Path| {
                std::fs::File::create(out_file)
                    .wrap_err("opening output file for writing")
                    .and_then(|mut out_file| {
                        std::io::copy(&mut decoder, &mut out_file)
                            .context("decoding base64 output and writing to a file")
                    })
                    .map(|_| {
                        println!("{}", out_file.display());
                    })
            };
            match (out_file, update_extension) {
                (Some(out_file), _) => {
                    save_to_file(&out_file)?;
                }
                (None, true) => {
                    save_to_file(&source_file.with_extension("pdf"))?;
                }
                _ => {
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
