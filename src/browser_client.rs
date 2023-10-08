use super::*;
use crate::geckodriver::{GeckodriverInstance, GeckodriverSpawnOptions};
use eyre::{Result, WrapErr};
use fantoccini::ClientBuilder;
use serde_json::{json, map::Map};
use std::hash::Hash;

#[derive(Debug)]
pub struct FirefoxInstance {
    pub client: fantoccini::Client,
}

pub struct FirefoxClient {
    _geckodriver: GeckodriverInstance,
    pub firefox: FirefoxInstance,
}

impl std::fmt::Debug for FirefoxClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", self._geckodriver).hash(&mut hasher);

        let hash = std::hash::Hasher::finish(&hasher);
        f.debug_struct(std::any::type_name::<Self>())
            .field("id", &format!("{hash:#01x}"))
            .finish_non_exhaustive()
    }
}

impl FirefoxClient {
    pub async fn kill(self) -> Result<()> {
        self.firefox
            .client
            .close()
            .await
            .wrap_err("killing the browser")
    }

    #[instrument(skip_all)]
    pub async fn spawn(options: &GeckodriverSpawnOptions, port: u16) -> Result<Self> {
        let geckodriver = GeckodriverInstance::spawn(options, port)
            .await
            .wrap_err("spawning geckodriver")?;
        let mut firefox = ClientBuilder::rustls();
        let mut caps = Map::new();
        let mut firefox_options = Map::new();
        if options.headless {
            firefox_options.insert("args".to_owned(), json!(["--headless"]));
        };
        caps.insert("moz:firefoxOptions".to_string(), firefox_options.into());
        trace!(?caps);
        firefox.capabilities(caps);
        debug!("spawning actual firefox");
        let firefox = firefox
            .connect(&format!("http://localhost:{port}"))
            .await
            .wrap_err("spawning firefox")
            .map(|client| FirefoxInstance { client })?;
        debug!("firefox spawned");

        Ok(Self {
            _geckodriver: geckodriver,
            firefox,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_log::test;

    #[test(tokio::test(flavor = "current_thread"))]
    async fn test_firefox_spawns() -> Result<()> {
        const _TEST_BIN: &[u8] = include_bytes!("../dependencies/geckodriver");
        let current_dir = std::env::current_dir()?;
        debug!(current_dir=%current_dir.display(), "spawning instance");
        let ff = FirefoxClient::spawn(
            &GeckodriverSpawnOptions {
                path: "../dependencies/geckodriver".into(),
                headless: true,
            },
            4444,
        )
        .await
        .wrap_err("spawning firefox")?;

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        ff.firefox.client.close().await?;

        Ok(())
    }
}
