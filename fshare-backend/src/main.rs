use axum::{
    Router,
    routing::{get, post},
};
use axum_cookie::CookieLayer;
use axum_keycloak_auth::{
    PassthroughMode, Url,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
};
use tower_http::{services::ServeDir, trace::TraceLayer};
// mod sessions;
mod storage;

mod redis_conn;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    redis_conn::init().await.unwrap();
    let keycloak_auth_instance = KeycloakAuthInstance::new(
        KeycloakConfig::builder()
            .server(Url::parse("https://auth.f-skipper.com/").unwrap())
            .realm(String::from("skippernet"))
            .build(),
    );

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/mkdir/{*path}", post(storage::mkdir))
                .route("/delete/{*path}", post(storage::delete))
                .route("/upload/{*path}", post(storage::upload))
                // .route(
                //     "/approve-session/{session_id}",
                //     get(sessions::approve_session),
                // )
                .layer(
                    KeycloakAuthLayer::<String>::builder()
                        .instance(keycloak_auth_instance)
                        .passthrough_mode(PassthroughMode::Block)
                        .expected_audiences(vec!["account".to_string()])
                        .build(),
                )
                .nest_service("/file", ServeDir::new("/public"))
                .route("/dir/", get(storage::get_root_dir))
                .route("/dir/{*path}", get(storage::get_dir)), // .route("/init-session", get(sessions::init_session))
                                                               // .route(
                                                               //     "/authenticate-session",
                                                               //     post(sessions::authenticate_session),
                                                               // )
                                                               // .route("/reload-session", get(sessions::reload_session))
                                                               // .route("/await-status", get(sessions::await_status)),
        )
        .layer(TraceLayer::new_for_http())
        // .layer(middleware::from_fn(sessions::inject_keycloak_token))
        .layer(CookieLayer::default());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
