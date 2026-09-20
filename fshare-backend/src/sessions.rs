use axum::{Json, extract::Path, http::HeaderMap};
use axum_anyhow::ApiResult;
use axum_cookie::{CookieManager, cookie::Cookie};
use tracing::info;
use uuid::Uuid;

use crate::redis_conn;
use redis::AsyncTypedCommands;

#[derive(serde::Serialize)]
pub struct InitSessionResp {
    approval_url: String,
}

pub async fn init_session(
    headers: HeaderMap,
    cookies: CookieManager,
) -> ApiResult<Json<InitSessionResp>> {
    let Some(host) = headers.get("x-forwarded-host") else {
        return Err(axum_anyhow::bad_request(
            "Failed to generate URL",
            "Missing origin header.",
        ));
    };

    let session_id = Uuid::new_v4();
    let approval_code = Uuid::new_v4();
    cookies.add(Cookie::new("session", session_id.to_string()));

    session_state::create_session(session_id, approval_code).await?;

    Ok(Json(InitSessionResp {
        approval_url: format!(
            "https://{}/approve_session/{}",
            host.to_str()?,
            approval_code
        ),
        // session_id: session_id.to_string(),
    }))
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SessionAuthentication {
    token: String,
    refresh: String,
}

pub async fn authenticate_session(
    cookies: CookieManager,
    Json(auth): Json<SessionAuthentication>,
) -> ApiResult<()> {
    // If authenticating existing session, use existing session id
    // If authenticating new session, generate a session id
    let session_id = match cookies.get("session") {
        Some(cookie) => cookie.value().parse()?,
        None => {
            let session_id = Uuid::new_v4();
            cookies.set(Cookie::new("session", session_id.to_string()));
            session_id
        }
    };
    let mut redis = redis_conn::get().await;

    info!("STORED SESSION FOR {session_id}",);
    redis
        .set(
            format!("SESSION/{session_id}"),
            format!("{}/{}", auth.token, auth.refresh),
        )
        .await?;

    Ok(())
}

pub async fn reload_session(cookies: CookieManager) -> ApiResult<Json<SessionAuthentication>> {
    let Some(session_cookie) = cookies.get("session") else {
        return Err(axum_anyhow::bad_request(
            "Missing session",
            "no session id found",
        ));
    };
    let session_id = session_cookie.value();

    let mut redis = redis_conn::get().await;
    let Some(pair) = redis.get(format!("SESSION/{session_id}")).await? else {
        return Err(axum_anyhow::not_found(
            "Session invalid",
            "session does not exist or has expired",
        ));
    };
    let (token, refresh) = pair.split_once("/").unwrap();

    info!("RELOADED SESSION {session_id}");

    return Ok(Json(SessionAuthentication {
        token: token.to_string(),
        refresh: refresh.to_string(),
    }));
}

pub async fn approve_session(
    cookies: CookieManager,
    Path(session_id): Path<String>,
) -> ApiResult<()> {
    let Ok(approval_code) = session_id.parse::<Uuid>() else {
        return Err(axum_anyhow::bad_request(
            "Invalid session id",
            "Failed to parse session id",
        ));
    };
    let approver_session = cookies.get("session").unwrap();
    let approver_session_id = approver_session.value().parse()?;

    return session_state::approve_session(approver_session_id, approval_code).await;
}

pub async fn await_status(cookies: CookieManager) -> ApiResult<()> {
    let Some(cookie) = cookies.get("session") else {
        return Err(axum_anyhow::not_found("Session not initialized", ""));
    };
    let session_id = cookie.value().parse()?;

    session_state::await_approval(session_id).await
}

mod session_state {
    use std::{sync::LazyLock, time::Duration};

    use axum_anyhow::ApiResult;
    use redis::AsyncTypedCommands;
    use tokio::sync::broadcast;
    use tracing::info;
    use uuid::Uuid;

    use crate::{redis_conn, sessions::SessionAuthentication};

    static SESSION_BROADCAST: LazyLock<broadcast::Sender<Uuid>> =
        LazyLock::new(|| broadcast::channel::<Uuid>(1024).0);

    pub async fn create_session(session_id: Uuid, approval_code: Uuid) -> anyhow::Result<()> {
        let mut redis = redis_conn::get().await;

        redis
            .set(format!("APPROVAL/{approval_code}"), session_id.to_string())
            .await?;

        Ok(())
    }

    pub async fn approve_session(approver_session: Uuid, approval_code: Uuid) -> ApiResult<()> {
        info!("Approving: {approval_code}");

        let mut redis = redis_conn::get().await;

        let Some(session_id) = redis.get(format!("APPROVAL/{approval_code}")).await? else {
            return Err(axum_anyhow::not_found(
                "Session not found",
                "Provided session id does not exist or has expired",
            ));
        };

        let pair = redis
            .get(format!("SESSION/{approver_session}"))
            .await?
            .unwrap();

        let set_success = redis.set_nx(format!("SESSION/{session_id}"), pair).await?;
        if !set_success {
            return Err(axum_anyhow::conflict(
                "Session already approved or rejected",
                "Cannot change status of session which has already been approved or rejected",
            ));
        }

        let _ = SESSION_BROADCAST.send(session_id.parse().unwrap()); // Allow for errors as it fails if there are no currently active receivers
        info!("Successfully approved {approval_code}");
        Ok(())
    }

    pub async fn await_approval(session_id: Uuid) -> ApiResult<()> {
        let broadcast_handle = tokio::spawn(wait_for_broadcast(session_id));
        let session_auth = get_status(session_id).await;

        if session_auth.is_some() {
            broadcast_handle.abort();
            return Ok(());
        }
        match tokio::time::timeout(Duration::from_secs(30), broadcast_handle).await {
            Ok(res) => {
                info!("BROADCAST MSG RECEIVED!");
                if res? {
                    return Ok(());
                } else {
                    return Err(axum_anyhow::not_found("Not approved yet", "Try again"));
                }
            }

            Err(_) => {
                //Timeout
                return Err(axum_anyhow::not_found("Timeout", "Try again"));
            }
        }
    }

    async fn wait_for_broadcast(session_id: Uuid) -> bool {
        let mut recv = SESSION_BROADCAST.subscribe();

        while let Ok(approved_id) = recv.recv().await {
            info!("BROADCAST CHECKING {approved_id} =? {session_id}");
            if approved_id == session_id {
                return true;
            }
        }

        return false;
    }

    async fn get_status(session_id: Uuid) -> Option<SessionAuthentication> {
        let mut redis = redis_conn::get().await;
        let pair = redis.get(format!("SESSION/{session_id}")).await.unwrap()?;
        let (token, refresh) = pair.split_once("/")?;
        return Some(SessionAuthentication {
            token: token.to_string(),
            refresh: refresh.to_string(),
        });
    }
}
