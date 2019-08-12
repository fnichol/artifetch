use crate::app::Data;
use crate::{Provider, Registry, Repo};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::SocketAddr;

pub enum RegistryConfig {
    GitHub {
        repos: Vec<Repo>,
        oauth_token: String,
    },
}

pub struct Config {
    pub bind_addr: SocketAddr,
    pub registry: HashMap<String, RegistryConfig>,
}

impl TryFrom<Config> for Data {
    type Error = crate::provider::github::client::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let mut registry = Registry::new();
        for (name, entry) in config.registry {
            match entry {
                RegistryConfig::GitHub { oauth_token, repos } => {
                    use crate::provider::github::GitHub;

                    registry.register(Provider::GitHub(GitHub::build(name, oauth_token, repos)?))
                }
            }
        }

        Ok(Self::new(registry))
    }
}
