use std::{path::PathBuf, time::Duration};

use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use axum_anyhow::ApiResult;
use axum_keycloak_auth::decode::KeycloakToken;
use redis::{AsyncTypedCommands, IntegerReplyOrNoOp};
use tracing::error;
use uuid::Uuid;

use crate::{redis_conn, storage};

fn redis_key(access_key: Uuid) -> String {
    format!("ACCESS_KEY/{access_key}")
}

const DEFAULT_ACCESS_KEY_DURATION: u64 = 5 * 60; // 5 minutes

pub async fn create(path: PathBuf, expiration: Option<Duration>) -> Uuid {
    let access_key = Uuid::new_v4();

    let mut redis = redis_conn::get().await;

    redis
        .set_ex(
            redis_key(access_key),
            path.to_str().unwrap(),
            expiration
                .map(|d| d.as_secs())
                .unwrap_or(DEFAULT_ACCESS_KEY_DURATION),
        )
        .await
        .unwrap();

    access_key
}

pub async fn get(access_key: Uuid) -> Option<(PathBuf, usize)> {
    let mut redis = redis_conn::get().await;
    let resp = redis.get(redis_key(access_key)).await.unwrap()?;
    let IntegerReplyOrNoOp::IntegerReply(ttl) = redis.ttl(redis_key(access_key)).await.unwrap()
    else {
        error!("Found stored access key but no expiration set. This is a BUG!");
        return None;
    };

    return Some((resp.parse().ok()?, ttl));
}

#[derive(serde::Serialize)]
pub struct NewAccessKey {
    key: Uuid,
}

#[derive(serde::Deserialize, Copy, Clone)]
pub struct AKDuration {
    duration: Option<u64>, //Seconds
}

pub async fn new_access_key(
    Path(path): Path<PathBuf>,
    Extension(token): Extension<KeycloakToken<String>>,
    duration: Query<AKDuration>,
) -> ApiResult<Json<NewAccessKey>> {
    let (_md, canon_path) = storage::get_md(path, Some(token.subject)).await?;

    return Ok(Json(NewAccessKey {
        key: create(
            canon_path,
            duration.duration.map(|d| Duration::from_secs(d)),
        )
        .await,
    }));
}
