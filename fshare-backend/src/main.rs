use axum::{Router, routing::get};
use tower_http::{services::ServeDir, trace::TraceLayer};
mod sessions;
mod storage;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .nest_service("/file", ServeDir::new("/public"))
                .route("/dir/", get(storage::get_root_dir))
                .route("/dir/{*path}", get(storage::get_dir))
                .route("/init-session", get(sessions::init_session))
                .route(
                    "/approve-session/{session_id}",
                    get(sessions::approve_session),
                )
                .route("/await-status/{session_id}", get(sessions::await_status)),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
