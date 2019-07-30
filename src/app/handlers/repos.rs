use crate::app::{self, paths};
use actix_web::{web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_repos_txt(
    path: web::Path<paths::Repos>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let provider = paths::get_provider(path.as_ref(), &data).expect("TODO: handle this");

    future::ok(
        HttpResponse::Ok().content_type("text/plain").body(
            provider
                .repos()
                .map(|r| format!("{}\n", r))
                .collect::<Vec<_>>()
                .join(""),
        ),
    )
}
