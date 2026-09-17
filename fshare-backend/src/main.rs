use axum::{Router, routing::get};
mod sessions;
mod storage;

#[tokio::main]
async fn main() {
    let app = Router::new().nest(
        "/api",
        Router::new()
            .route("/storage/{*path}", get(storage::get_from_storage))
            .route("/init-session", get(sessions::init_session))
            .route(
                "/approve-session/{session_id}",
                get(sessions::approve_session),
            )
            .route("/await-status/{session_id}", get(sessions::await_status)), // .layer(middleware::from_fn(cors::cors_middleware))
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
