use std::path::PathBuf;

use redis::AsyncTypedCommands;
use uuid::Uuid;

use crate::redis_conn;

fn redis_key(access_key: Uuid) -> String {
    format!("ACCESS_KEY/{access_key}")
}

const ACCESS_KEY_DURATION: u64 = 5 * 60;

pub async fn create(path: PathBuf) -> Uuid {
    let access_key = Uuid::new_v4();

    let mut redis = redis_conn::get().await;

    redis
        .set_ex(
            redis_key(access_key),
            path.to_str().unwrap(),
            ACCESS_KEY_DURATION,
        )
        .await
        .unwrap();

    access_key
}

pub async fn get(access_key: Uuid) -> Option<PathBuf> {
    let mut redis = redis_conn::get().await;
    let resp = redis.get(redis_key(access_key)).await.unwrap()?;
    return resp.parse().ok();
}
