use axum::{
    Json,
    extract::{Path, Request},
};
use axum_anyhow::ApiResult;
use uuid::Uuid;

use crate::sessions::session_state::ApprovalStatus;

#[derive(serde::Serialize)]
pub struct InitSessionResp {
    approval_url: String,
    session_id: String,
}

pub async fn init_session(req: Request) -> ApiResult<Json<InitSessionResp>> {
    // println!("REQ: {req:#?}");

    let Some(host) = req.headers().get("x-forwarded-host") else {
        return Err(axum_anyhow::bad_request(
            "Failed to generate URL",
            "Missing origin header.",
        ));
    };

    let session_id = uuid::Uuid::new_v4();

    session_state::create_session(session_id).await;

    Ok(Json(InitSessionResp {
        approval_url: format!("https://{}/approve_session/{}", host.to_str()?, session_id),
        session_id: session_id.to_string(),
    }))
}

pub async fn approve_session(Path(session_id): Path<String>,req: Request) -> ApiResult<()> {
    println!("APPROVER REQ: {req:#?}");
    let Ok(session_id) = session_id.parse::<Uuid>() else {
        return Err(axum_anyhow::bad_request(
            "Invalid session id",
            "Failed to parse session id",
        ));
    };

    return session_state::approve_session(session_id).await;
}

pub async fn await_status(Path(session_id): Path<String>) -> ApiResult<Json<ApprovalStatus>> {
    let Ok(session_id) = session_id.parse::<Uuid>() else {
        return Err(axum_anyhow::bad_request(
            "Invalid session id",
            "Failed to parse session id",
        ));
    };

    session_state::await_approval(session_id)
        .await
        .map(|status| Json(status))
}

mod session_state {
    use std::{collections::HashMap, sync::LazyLock, time::Duration};

    use axum_anyhow::ApiResult;
    use tokio::sync::{Mutex, broadcast};
    use uuid::Uuid;

    static APPROVED_SESSIONS: LazyLock<Mutex<HashMap<Uuid, ApprovalStatus>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    static SESSION_BROADCAST: LazyLock<broadcast::Sender<Uuid>> =
        LazyLock::new(|| broadcast::channel::<Uuid>(1024).0);

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub enum ApprovalStatus {
        Pending,
        Approved,
        #[allow(unused)]
        Rejected,
        Missing,
    }

    pub async fn create_session(session_id: Uuid) {
        APPROVED_SESSIONS
            .lock()
            .await
            .insert(session_id, ApprovalStatus::Pending);
    }

    pub async fn approve_session(session_id: Uuid) -> ApiResult<()> {
        let mut session_map = APPROVED_SESSIONS.lock().await;

        let Some(entry) = session_map.get_mut(&session_id) else {
            return Err(axum_anyhow::not_found(
                "Session not found",
                "Provided session id does not exist or has expired",
            ));
        };

        if *entry != ApprovalStatus::Pending && *entry != ApprovalStatus::Approved {
            return Err(axum_anyhow::conflict(
                "Session already approved or rejected",
                "Cannot change status of session which has already been approved or rejected",
            ));
        }

        *entry = ApprovalStatus::Approved;
        let _ = SESSION_BROADCAST.send(session_id); // Allow for errors as it fails if there are no currently active receivers

        drop(session_map);
        Ok(())
    }

    pub async fn await_approval(session_id: Uuid) -> ApiResult<ApprovalStatus> {
        let broadcast_handle = tokio::spawn(wait_for_broadcast(session_id));
        let current_status = get_status(session_id).await;

        match current_status {
            ApprovalStatus::Pending => {
                match tokio::time::timeout(Duration::from_secs(60), broadcast_handle).await {
                    Ok(broadcast_status) => match broadcast_status {
                        Ok(approved) => {
                            if approved {
                                Ok(ApprovalStatus::Approved)
                            } else {
                                eprintln!("Warning: Session broadcast lagged!");
                                Ok(ApprovalStatus::Pending)
                            }
                        }
                        Err(join_error) => {
                            let error_id = Uuid::new_v4();
                            eprintln!(
                                "Error: ({error_id}) Failed to join on session approval broadcast with error: {join_error:#?}"
                            );

                            Err(axum_anyhow::internal_error(
                                "Internal server error",
                                &format!("EID: {error_id}"),
                            ))
                        }
                    },
                    Err(_) => {
                        //Timeout, retry from web
                        Ok(ApprovalStatus::Pending)
                    }
                }
            }
            status => {
                broadcast_handle.abort();
                Ok(status)
            }
        }
    }

    async fn wait_for_broadcast(session_id: Uuid) -> bool {
        let mut recv = SESSION_BROADCAST.subscribe();

        while let Ok(approved_id) = recv.recv().await {
            if approved_id == session_id {
                return true;
            }
        }

        false
    }

    async fn get_status(session_id: Uuid) -> ApprovalStatus {
        let guard = APPROVED_SESSIONS.lock().await;

        let Some(status) = guard.get(&session_id) else {
            return ApprovalStatus::Missing;
        };
        return status.clone();
    }
}
