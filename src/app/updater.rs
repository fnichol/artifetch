use super::Data;
use crate::Repo;
use actix_web::web;
use futures::{future::Future, stream::Stream};
use log::info;
use rand::Rng;
use std::fmt;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_timer::{Delay, Interval};

pub fn spawn(updater: RepoUpdater) {
    let initial_updater = updater.clone();
    actix_rt::spawn(
        Delay::new(Instant::now())
            .and_then(move |_| {
                info!("populating; repo={}", &initial_updater);
                initial_updater.update().expect("TODO: handle this");
                Ok(())
            })
            .map_err(|e| panic!("TODO: updater errored; err={:?}", e)),
    );

    actix_rt::spawn(
        Interval::new(
            Instant::now() + rand_splay_delay() + updater.interval(),
            updater.interval(),
        )
        .for_each(move |_| {
            info!("updating; repo={}", &updater);
            updater.update().expect("TODO: handle this");
            Ok(())
        })
        .map_err(|e| panic!("TODO: updater errored; err={:?}", e)),
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
    pub fn new<D, O, N>(data: web::Data<Data>, domain: D, owner: O, name: N) -> io::Result<Self>
    where
        D: Into<String>,
        O: Into<String>,
        N: Into<String>,
    {
        let domain = domain.into();
        let owner = owner.into();
        let name = name.into();

        Ok(Self {
            data,
            domain,
            owner,
            name,
        })
    }

    pub fn interval(&self) -> Duration {
        self.repo().interval()
    }

    pub fn update(&self) -> io::Result<()> {
        // self.provider().update_repo(&self.owner, &self.name, update)
        Ok(())
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
        write!(f, "{}({}/{})", self.domain, self.owner, self.name)
    }
}
