use std::{env, ops::Deref, sync::Arc};

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use axum_keycloak_auth::{
    PassthroughMode, Url,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
};
use sea_orm::{Database, DatabaseConnection};
use tower_http::trace::TraceLayer;

mod access_keys;
mod db;
mod entry_data;

#[derive(Debug, Clone)]
pub struct AppState {
    db: DatabaseConnection,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .init();

    let db_fut = tokio::spawn(async {
        Database::connect(env::var("DATABASE_URL").unwrap().deref())
            .await
            .unwrap()
    });

    let keycloak_auth_instance_fut = tokio::spawn(async {
        Arc::new(KeycloakAuthInstance::new(
            KeycloakConfig::builder()
                .server(Url::parse("https://auth.f-skipper.com/").unwrap())
                .realm(String::from("skippernet"))
                .build(),
        ))
    });

    let db = db_fut.await.unwrap();
    let keycloak_auth_instance = keycloak_auth_instance_fut.await.unwrap();

    let appstate = AppState { db };

    let app = Router::new()
        // .route("/upload/{*path}", post(storage::upload))
        .layer(DefaultBodyLimit::max(2_000_000_000))
        // .route("/mkdir/{*path}", post(storage::mkdir))
        // .route("/delete/{*path}", post(storage::delete))
        // .route("/rename/{*path}", post(storage::rename))
        // .route("/access_key/{*path}", post(access_keys::new_access_key))
        .layer(
            KeycloakAuthLayer::<String>::builder()
                .instance(keycloak_auth_instance.clone())
                .passthrough_mode(PassthroughMode::Block)
                .expected_audiences(vec!["account".to_string()])
                .build(),
        )
        // .nest_service(
        //     "/thumbnail",
        //     Router::new()
        //         .fallback_service(ServeDir::new("/thumbnails"))
        //         .layer(middleware::from_fn(storage::prepare_thumbnail)),
        // )
        // .nest_service(
        //     "/file",
        //     Router::new()
        //         .fallback_service(ServeDir::new("/"))
        //         .layer(middleware::from_fn(storage::prepare_file_req)),
        // )
        .route("/file/{*path}", get(entry_data::get_file))
        .route("/dir/", get(entry_data::get_root_dir))
        .route("/dir/{*path}", get(entry_data::get_dir))
        .layer(
            KeycloakAuthLayer::<String>::builder()
                .instance(keycloak_auth_instance.clone())
                .passthrough_mode(PassthroughMode::Pass)
                .expected_audiences(vec!["account".to_string()])
                .build(),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(appstate);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
