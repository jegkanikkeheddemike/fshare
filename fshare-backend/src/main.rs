use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use axum_cookie::CookieLayer;
use axum_keycloak_auth::{
    PassthroughMode, Url,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
};
use tower_http::{services::ServeDir, trace::TraceLayer};

mod redis_conn;
mod storage;
mod access_keys;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let keycloak_auth_instance = Arc::new(KeycloakAuthInstance::new(
        KeycloakConfig::builder()
            .server(Url::parse("https://auth.f-skipper.com/").unwrap())
            .realm(String::from("skippernet"))
            .build(),
    ));

    redis_conn::init().await.unwrap();

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/upload/{*path}", post(storage::upload))
                .layer(DefaultBodyLimit::max(2_000_000_000))
                .route("/mkdir/{*path}", post(storage::mkdir))
                .route("/delete/{*path}", post(storage::delete))
                .route("/rename/{*path}", post(storage::rename))
                .layer(
                    KeycloakAuthLayer::<String>::builder()
                        .instance(keycloak_auth_instance.clone())
                        .passthrough_mode(PassthroughMode::Block)
                        .expected_audiences(vec!["account".to_string()])
                        .build(),
                )
                .nest_service(
                    "/thumbnail",
                    Router::new()
                        .fallback_service(ServeDir::new("/thumbnails"))
                        .layer(middleware::from_fn(storage::prepare_thumbnail)),
                )
                .nest_service(
                    "/file",
                    Router::new()
                        .fallback_service(ServeDir::new("/"))
                        .layer(middleware::from_fn(storage::prepare_file_req)),
                )
                .route("/dir/", get(storage::get_root_dir))
                .route("/dir/{*path}", get(storage::get_dir)),
        )
        .layer(
            KeycloakAuthLayer::<String>::builder()
                .instance(keycloak_auth_instance.clone())
                .passthrough_mode(PassthroughMode::Pass)
                .expected_audiences(vec!["account".to_string()])
                .build(),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CookieLayer::default());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
