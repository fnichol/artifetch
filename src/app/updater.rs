use super::Data;
use crate::{provider, Repo};
use actix_web::web;
use futures::{Future, Stream};
use log::{error, info, warn};
use rand::Rng;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_timer::{Delay, Interval};

pub fn spawn(updater: RepoUpdater) {
    let updater = Arc::new(updater);
    let initial_updater = updater.clone();

    actix_rt::spawn(
        Delay::new(Instant::now())
            .map_err(|e| panic!("TODO: tokio_timer errored; err={:?}", e))
            .and_then(move |_| {
                info!("populating repo; {}", &initial_updater);
                let uerr = initial_updater.clone();
                initial_updater.update().map_err(move |err| {
                    error!("populate failed; {}, err={}", uerr, err);
                })
            }),
    );

    actix_rt::spawn(
        Interval::new(
            Instant::now() + rand_splay_delay() + updater.interval(),
            updater.interval(),
        )
        .map_err(|e| panic!("TODO: tokio_timer errored; err={:?}", e))
        .for_each(move |_| {
            info!("updating repo; {}", &updater);
            let uerr = updater.clone();
            updater.update().map_err(move |err| {
                warn!("update failed; {}, err={}", uerr, err);
            })
        }),
    );
}

fn rand_splay_delay() -> Duration {
    let mut rng = rand::thread_rng();

    Duration::from_secs(rng.gen_range(0, 30))
}

#[derive(Clone, Debug)]
pub struct RepoUpdater {
    data: web::Data<Data>,
    domain: String,
    owner: String,
    name: String,
}

impl RepoUpdater {
    pub fn new<D, O, N>(data: web::Data<Data>, domain: D, owner: O, name: N) -> Self
    where
        D: Into<String>,
        O: Into<String>,
        N: Into<String>,
    {
        let domain = domain.into();
        let owner = owner.into();
        let name = name.into();

        Self {
            data,
            domain,
            owner,
            name,
        }
    }

    pub fn interval(&self) -> Duration {
        self.repo().interval()
    }

    pub fn update(&self) -> impl Future<Item = (), Error = provider::Error> {
        self.data
            .provider(&self.domain)
            .expect("provider domain should exist")
            .update_repo(self.owner.clone(), self.name.clone())
    }

    fn repo(&self) -> Arc<Repo> {
        self.data
            .provider(&self.domain)
            .expect("provider domain should exist")
            .repo(&self.owner, &self.name)
            .expect("repo should exist in provider")
    }
}

impl fmt::Display for RepoUpdater {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "domain={}, repo={}/{}",
            self.domain, self.owner, self.name
        )
    }
}
