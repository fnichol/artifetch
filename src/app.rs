use crate::provider;
use actix_web::{middleware, web, App, HttpServer};
use data::Data;
use handlers::{assets, providers, releases, repos, targets};
use std::convert::TryFrom;
use std::error;
use std::fmt;
use std::io;
use std::net::ToSocketAddrs;
use updater::RepoUpdater;

pub use config::{config, Config};

mod config;
mod data;
mod handlers;
mod paths;
mod updater;

pub fn run(config: Config) -> Result<(), Error> {
    let addr = config.bind_addr;
    let data = web::Data::new(Data::try_from(config)?);

    let sys = actix_rt::System::new(env!("CARGO_PKG_NAME"));
    schedule_updaters(data.clone());
    start_server(addr, data)?;
    Ok(sys.run()?)
}

fn schedule_updaters(data: web::Data<Data>) {
    for provider in data.providers() {
        for repo in provider.repos() {
            updater::spawn(RepoUpdater::new(
                data.clone(),
                provider.domain(),
                repo.owner(),
                repo.name(),
            ));
        }
    }
}

fn start_server<A: ToSocketAddrs>(addr: A, data: web::Data<Data>) -> Result<(), Error> {
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .register_data(data.clone())
            .configure(routes)
    })
    .bind(addr)?
    .start();

    Ok(())
}

fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/v1").configure(providers));
}

fn providers(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/providers.txt")
            .service(web::resource("").route(web::get().to_async(providers::get_providers_txt))),
    )
    .service(web::scope("/providers").service(web::scope("{provider}").configure(repos)));
}

fn repos(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/repos.txt")
            .service(web::resource("").route(web::get().to_async(repos::get_repos_txt))),
    )
    .service(
        web::scope("/repos")
            .service(web::scope("/{owner}").service(web::scope("/{repo}").configure(releases))),
    );
}

fn releases(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/releases.txt")
            .service(web::resource("").route(web::get().to_async(releases::get_releases_txt))),
    )
    .service(web::scope("/releases").service(web::scope("/{version}").configure(targets)));
}

fn targets(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/targets.txt")
            .service(web::resource("").route(web::get().to_async(targets::get_targets_txt))),
    )
    .service(web::scope("/targets").service(web::scope("/{target}").configure(assets)));
}

fn assets(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/assets.txt")
            .service(web::resource("").route(web::get().to_async(assets::get_assets_txt))),
    )
    .service(
        web::scope("/assets").service(
            web::scope("/{asset}")
                .service(web::resource("").route(web::get().to_async(assets::get_asset))),
        ),
    );
}

#[derive(Debug)]
pub enum Error {
    Config(provider::Error),
    ConfigLoad(Box<dyn error::Error + Send + Sync>),
    RepoConfig(&'static str),
    ServerInit(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Config(ref err) => err.fmt(f),
            Error::ConfigLoad(ref err) => err.fmt(f),
            Error::RepoConfig(ref msg) => write!(f, "{}", msg),
            Error::ServerInit(ref err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Config(ref err) => err.source(),
            Error::ConfigLoad(ref err) => err.source(),
            Error::RepoConfig(_) => None,
            Error::ServerInit(ref err) => err.source(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::ServerInit(err)
    }
}

impl From<provider::Error> for Error {
    fn from(err: provider::Error) -> Self {
        Error::Config(err)
    }
}
