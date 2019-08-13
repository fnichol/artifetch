use crate::app::{self, paths};
use actix_web::{web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_targets_txt(
    path: web::Path<paths::Targets>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    future::result(paths::get_release(path.as_ref(), &data)).and_then(|release| {
        HttpResponse::Ok().content_type("text/plain").body(
            release
                .targets()
                .map(|t| format!("{}\n", t))
                .collect::<Vec<_>>()
                .join(""),
        )
    })
}
