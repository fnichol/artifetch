use crate::app::{self, paths};
use actix_web::{web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_releases_txt(
    path: web::Path<paths::Releases>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    future::result(paths::get_repo(path.as_ref(), &data)).and_then(|repo| {
        HttpResponse::Ok().content_type("text/plain").body(
            repo.releases()
                .map(|r| format!("{}\n", r.name()))
                .collect::<Vec<_>>()
                .join(""),
        )
    })
}
