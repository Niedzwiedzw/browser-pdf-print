use super::*;
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
pub static GECKODRIVER_BIN: &[u8] = include_bytes!("../dependencies/geckodriver");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeckodriverSpawnOptions {
    pub path: PathBuf,
    pub headless: bool,
}

impl std::default::Default for GeckodriverSpawnOptions {
    fn default() -> Self {
        Self {
            path: "./dependencies/geckodriver".into(),
            headless: false,
        }
    }
}

#[derive(Debug)]
pub struct GeckodriverInstance {
    pub port: u16,
    _instance: Child,
}

impl GeckodriverInstance {
    #[instrument]
    pub async fn spawn(
        GeckodriverSpawnOptions { path, .. }: &GeckodriverSpawnOptions,
        port: u16,
    ) -> Result<Self> {
        let mut instance = tokio::process::Command::new(path)
            .args(["--port", &port.to_string()])
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .wrap_err("spawning geckodriver")?;
        if let Some(stderr) = instance.stderr.take() {
            tokio::task::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Some(line) = reader.next_line().await.ok().and_then(|v| v) {
                    warn!("{line}");
                }
            });
        }
        if let Some(stdout) = instance.stdout.take() {
            tokio::task::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Some(line) = reader.next_line().await.ok().and_then(|v| v) {
                    debug!("{line}");
                }
            });
        }
        debug!("waiting for gecko to spin up");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(GeckodriverInstance {
            port,
            _instance: instance,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_print_default_geckodriver_config() -> Result<()> {
        println!("{}", toml::to_string(&GeckodriverSpawnOptions::default())?);
        Ok(())
    }
}
