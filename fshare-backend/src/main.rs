use axum::{Router, http::HeaderValue, middleware, routing::get};

mod sessions;
mod cors;

#[tokio::main]
async fn main() {

    let app = Router::new()
        .route("/init-session", get(sessions::init_session))
        .layer(middleware::from_fn(cors::cors_middleware));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9300").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
