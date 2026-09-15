use std::net::IpAddr;
use std::sync::LazyLock;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, Response},
    middleware::Next,
    response::IntoResponse,
};

pub async fn cors_middleware(req: Request, next: Next) -> Response<Body> {
    let forbidden =
        axum_anyhow::forbidden("Forbidden", "Host header is missing or invalid").into_response();

    let Some(origin) = req.headers().get("origin") else {
        return forbidden;
    };
    let Ok(origin_str) = origin.to_str() else {
        return forbidden;
    };

    static LOCAL_IP: LazyLock<String> = LazyLock::new(|| {
        format!(
            "http://{}:5173",
            local_ip_address::local_ip().unwrap().to_string()
        )
    });

    let allowed_origins = &[
        "http://localhost:5173",
        "https://files.f-skipper.com",
        &LOCAL_IP,
    ];

    let Some(valid_origin) = allowed_origins.iter().find(|o| *o == &origin_str) else {
        return forbidden;
    };

    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        "Access-Control-Allow-Origin",
        valid_origin.parse::<HeaderValue>().unwrap(),
    );

    resp
}
