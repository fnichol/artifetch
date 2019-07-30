use crate::app::Data;
use crate::{Provider, Registry, Repo};
use std::collections::HashMap;
use std::net::SocketAddr;

pub enum RegistryConfig {
    GitHub { repos: Vec<Repo> },
}

pub struct Config {
    pub bind_addr: SocketAddr,
    pub registry: HashMap<String, RegistryConfig>,
}

impl From<Config> for Data {
    fn from(config: Config) -> Self {
        let mut registry = Registry::new();
        for (name, entry) in config.registry {
            match entry {
                RegistryConfig::GitHub { repos } => {
                    use crate::provider::github::GitHub;

                    registry.register(Provider::GitHub(GitHub::new(name, repos)))
                }
            }
        }

        Data { registry }
    }
}
