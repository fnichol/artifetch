use crate::app::{self, paths};
use actix_web::{web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_releases_txt(
    path: web::Path<paths::Releases>,
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let repo = paths::get_repo(path.as_ref(), &data).expect("TODO: handle this");

    future::ok(
        HttpResponse::Ok().content_type("text/plain").body(
            repo.releases()
                .map(|r| format!("{}\n", r))
                .collect::<Vec<_>>()
                .join(""),
        ),
    )
}
