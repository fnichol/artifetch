use actix_web::{middleware, web, App, HttpServer};
use config::Config;
use data::Data;
use handlers::{assets, providers, releases, repos, targets};
use std::io;
use std::net::ToSocketAddrs;
use updater::RepoUpdater;

pub mod config;
mod data;
mod handlers;
mod paths;
mod updater;

pub fn run(config: Config) -> io::Result<()> {
    let addr = config.bind_addr;
    let data = web::Data::new(stub_data(config.into()));

    let sys = actix_rt::System::new("ghrr");
    schedule_updaters(data.clone())?;
    start_server(addr, data)?;
    sys.run()
}

fn schedule_updaters(data: web::Data<Data>) -> io::Result<()> {
    for provider in data.providers() {
        for repo in provider.repos() {
            updater::spawn(RepoUpdater::new(
                data.clone(),
                provider.domain(),
                repo.owner(),
                repo.name(),
            )?);
        }
    }

    Ok(())
}

fn start_server<A: ToSocketAddrs>(addr: A, data: web::Data<Data>) -> io::Result<()> {
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

// TODO: remove
fn stub_data(mut data: Data) -> Data {
    use crate::{Asset, Release, Target};
    use actix_web::http::Uri;

    data.provider_mut("github.com")
        .expect("provider should be registered")
        .update_repo("fnichol", "names", |repo| {
            let mut target = Target::new("darwin-x86_64");
            target.set_assets(vec![Asset::new(
                "names",
                Uri::from_static("https://github.com/fnichol/names/releases/download/v0.11.0/names_0.11.0_darwin_x86_64.zip")
            )]);
            let mut release = Release::new("v0.11.0");
            release.set_targets(vec![target, Target::new("linux-x86_64")]);

            repo.set_releases(vec![release]);
        })
        .expect("repo should exist");

    data
}
