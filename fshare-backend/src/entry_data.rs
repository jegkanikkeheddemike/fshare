use std::ops::Deref;

use axum::{
    Json,
    extract::{Path, Request, State},
    response::IntoResponse,
};
use axum_anyhow::ApiResult;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{error, info};
use uuid::Uuid;

use crate::{AppState, db::entries};

#[derive(Debug, serde::Serialize)]
pub struct EntryInfo {
    relative_name: String,
    is_dir: bool,
    mime: Option<String>,
    access_key: Option<Uuid>,
}

#[derive(Debug, serde::Serialize)]
pub struct DirInfo {
    entries: Vec<EntryInfo>,
}

fn not_found<T>() -> ApiResult<T> {
    return Err(axum_anyhow::not_found(
        "Not found",
        "No item or directory at this path",
    ));
}

pub async fn get_root_dir(appstate: State<AppState>) -> ApiResult<Json<DirInfo>> {
    get_dir(appstate, Path("".into())).await
}

struct Ownerlist<'a>(&'a [(&'a str, &'a str)]);

impl<'a> Into<Condition> for Ownerlist<'a> {
    fn into(self) -> Condition {
        self.0
            .iter()
            .fold(Condition::any(), |c, (owner, owner_type)| {
                c.add(
                    Condition::all()
                        .add(entries::Column::Owner.eq(*owner))
                        .add(entries::Column::OwnerType.eq(*owner_type)),
                )
            })
    }
}

pub async fn get_dir(
    State(appstate): State<AppState>,
    Path(dir_path): Path<String>,
) -> ApiResult<Json<DirInfo>> {
    // We need it prepended with "/"
    let dir_name = format!("/{dir_path}");

    info!("Dir lookup in {dir_name}");

    let ownerlist = Ownerlist(&[("public", "public")]);

    let db_entries = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir_name))
        .filter::<Condition>(ownerlist.into())
        .all(&appstate.db)
        .await
        .map_err(|err| axum_anyhow::internal_error("Internal error", &err.to_string()))?;

    let dir = db_entries
        .into_iter()
        .map(|e| {
            let mime = if e.is_dir {
                None
            } else {
                mime_guess::from_path(&e.name)
                    .first()
                    .map(|m| m.essence_str().to_owned())
            };

            EntryInfo {
                relative_name: e.name,
                is_dir: e.is_dir,
                mime,
                access_key: None,
            }
        })
        .collect::<Vec<_>>();

    return Ok(Json(DirInfo { entries: dir }));
}

pub async fn get_file(
    State(appstate): State<AppState>,
    Path(file_path): Path<String>,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (dir, filename) = match file_path.rsplit_once("/") {
        Some((dir, file)) => (format!("/{dir}/"), file),
        None => {
            // In root dir
            ("/".to_owned(), file_path.deref())
        }
    };

    info!("File lookup at {dir} {filename}");

    let ownerlist = Ownerlist(&[("public", "public")]);

    let file = entries::Entity::find()
        .filter(entries::Column::Dir.eq(&dir))
        .filter(entries::Column::Name.eq(filename))
        .filter::<Condition>(ownerlist.into())
        .one(&appstate.db)
        .await?;

    let Some(file_entry) = file else {
        return not_found();
    };

    let storage_path = match (file_entry.is_dir, &file_entry.storage_path) {
        (false, Some(storage_path)) => storage_path,
        (true, None) => {
            // Is dir
            return not_found();
        }
        _ => {
            error!("Inconsisdent data for entry: {file_entry:#?}");
            return Err(axum_anyhow::internal_error(
                "Internal error",
                "Try again later",
            ));
        }
    };

    Ok(ServeFile::new(format!("/data/{storage_path}"))
        .oneshot(request)
        .await?)
}
