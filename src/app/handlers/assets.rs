use crate::app::{self, paths};
use actix_web::{http, web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_assets_txt(
    path: web::Path<paths::Assets>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let target = paths::get_target(path.as_ref(), &data).expect("TODO: handle this");

    future::ok(
        HttpResponse::Ok().content_type("text/plain").body(
            target
                .assets()
                .map(|a| format!("{}\n", a))
                .collect::<Vec<_>>()
                .join(""),
        ),
    )
}

pub fn get_asset(
    path: web::Path<paths::Asset>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let asset = paths::get_asset(path.as_ref(), &data).expect("TODO: handle this");

    future::ok(
        HttpResponse::Found()
            .header(http::header::LOCATION, asset.download_uri().to_string())
            .finish(),
    )
}
