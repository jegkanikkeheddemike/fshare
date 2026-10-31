use std::{env, ops::Deref, sync::Arc};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use axum_anyhow::ApiError;
use axum_keycloak_auth::{
    PassthroughMode, Url,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
};
use sea_orm::{Database, DatabaseConnection};
use tower_http::trace::TraceLayer;
use tracing::error;

mod access_key_utils;
mod db;
mod entry_data;

#[derive(Debug, Clone)]
pub struct AppState {
    db: DatabaseConnection,
}

pub fn not_found() -> ApiError {
    return axum_anyhow::not_found("Not found", "No item or directory at this path");
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt()
            .pretty()
            .with_file(true)
            .with_level(true)
            .with_line_number(true)
            .init();

        axum_anyhow::on_error(|err| {
            error!("Error: {err:?}");
        });
    }
    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::fmt()
            .with_file(true)
            .with_level(true)
            .with_line_number(true)
            .init();

        axum_anyhow::on_error(|err| {
            if err.status().is_server_error() {
                error!("Internal error: {err:#?}");
            }
        });
    }

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
    #[cfg(not(debug_assertions))]
    {
        use migration::MigratorTrait;
        migration::Migrator::up(&db, None).await.unwrap();
    }

    let keycloak_auth_instance = keycloak_auth_instance_fut.await.unwrap();

    let appstate = AppState { db };

    let app = Router::new()
        .route("/upload/{*path}", post(entry_data::upload))
        .layer(DefaultBodyLimit::max(2_000_000_000))
        .route("/mkdir/{*path}", post(entry_data::mkdir))
        .route("/delete/{*path}", post(entry_data::delete))
        .route("/rename/{*path}", post(entry_data::rename))
        // .route("/access_key/{*path}", post(access_keys::new_access_key))
        .layer(
            KeycloakAuthLayer::<String>::builder()
                .instance(keycloak_auth_instance.clone())
                .passthrough_mode(PassthroughMode::Block)
                .expected_audiences(vec!["account".to_string()])
                .build(),
        )
        .route("/thumbnail/{*path}", get(entry_data::get_thumbnail))
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
