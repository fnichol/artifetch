use crate::app;
use actix_web::{web, Error, HttpResponse};
use futures::{future, Future};

pub fn get_providers_txt(
    data: web::Data<app::Data>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    future::ok(
        HttpResponse::Ok().content_type("text/plain").body(
            data.providers()
                .map(|p| format!("{}\n", p.domain()))
                .collect::<Vec<_>>()
                .join(""),
        ),
    )
}
