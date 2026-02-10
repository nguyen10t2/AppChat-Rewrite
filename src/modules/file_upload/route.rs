use actix_web::web;

use crate::modules::file_upload::repository::FileRepository;

pub fn configure<R>(cfg: &mut web::ServiceConfig)
where
    R: FileRepository + Send + Sync + 'static,
{
    cfg.service(
        web::resource("/upload")
            .route(web::post().to(crate::modules::file_upload::handle::upload_file::<R>)),
    )
    .service(
        web::resource("/{file_id}")
            .route(web::get().to(crate::modules::file_upload::handle::get_file::<R>))
            .route(web::delete().to(crate::modules::file_upload::handle::delete_file::<R>)),
    );
}
