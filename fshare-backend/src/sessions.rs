use axum::extract::Request;
use axum_anyhow::ApiResult;

pub async fn init_session(req: Request) -> ApiResult<String> {
    let Some(host) = req.headers().get("origin") else {
        return Err(axum_anyhow::bad_request(
            "Failed to generate URL",
            "Missing host header.",
        ));
    };

    Ok(format!(
        "{}/approve_session/{}",
        host.to_str()?,
        uuid::Uuid::new_v4()
    ))
}

pub async fn approve_session() -> ApiResult<String> {
    Ok("".into())
}
