use axum::{
    Json,
    body::Body,
    extract::{Path, Request, State},
    response::IntoResponse,
};
use axum_anyhow::{ApiError, ApiResult};
use futures::StreamExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::{self, Set}, ColumnTrait, Condition, EntityTrait, ModelTrait, QueryFilter, QueryOrder, sqlx::types::chrono,
};
use thumbnails::Thumbnailer;
use tokio::io::AsyncWriteExt;
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

fn not_found() -> ApiError {
    return axum_anyhow::not_found("Not found", "No item or directory at this path");
}

pub async fn get_root_dir(appstate: State<AppState>) -> ApiResult<Json<DirInfo>> {
    get_dir(appstate, Path("".into())).await
}

struct Ownerlist<'a>(&'a [(&'a str, &'a str)]);

impl<'a> Ownerlist<'a> {
    fn to_condition(self) -> Condition {
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

fn split_name<'a>(path: &'a str) -> (&'a str, &'a str) {
    match path.rsplit_once("/") {
        Some((dir, file)) => (dir.trim_end_matches('/'), file),
        None => {
            // In root dir
            ("", path.trim_end_matches('/'))
        }
    }
}

fn data_path(path: &str) -> String {
    format!("/data/{path}")
}

async fn get_dir_entries(appstate: &AppState, dir_name: &str) -> ApiResult<Vec<entries::Model>> {
    let ownerlist = Ownerlist(&[("public", "public")]);

    entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir_name))
        .filter(ownerlist.to_condition())
        .order_by_asc(entries::Column::Name)
        .all(&appstate.db)
        .await
        .map_err(|err| axum_anyhow::internal_error("Internal error", &err.to_string()))
}

pub async fn get_dir(
    State(appstate): State<AppState>,
    Path(dir_name): Path<String>,
) -> ApiResult<Json<DirInfo>> {
    let dir_name = dir_name.trim_matches('/');
    info!("Dir lookup in {dir_name}");

    let db_entries = get_dir_entries(&appstate, dir_name).await?;

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
    Path(path): Path<String>,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (dir, name) = split_name(&path);

    info!("File lookup at {dir} {name}");

    let ownerlist = Ownerlist(&[("public", "public")]);

    let file = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(ownerlist.to_condition())
        .one(&appstate.db)
        .await?;

    let Some(file_entry) = file else {
        return Err(not_found());
    };

    let storage_path = match (file_entry.is_dir, &file_entry.storage_path) {
        (false, Some(storage_path)) => storage_path,
        (true, None) => {
            // Is dir
            return Err(not_found());
        }
        _ => {
            error!("Inconsisdent data for entry: {file_entry:#?}");
            return Err(axum_anyhow::internal_error(
                "Internal error",
                "Try again later",
            ));
        }
    };

    Ok(ServeFile::new(data_path(&storage_path))
        .oneshot(request)
        .await?)
}

pub async fn get_thumbnail(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (dir, name) = split_name(&path);

    let ownerlist = Ownerlist(&[("public", "public")]);

    let file = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(ownerlist.to_condition())
        .one(&appstate.db)
        .await?;

    let Some(file_entry) = file else {
        return Err(not_found());
    };

    let Some(thumbnail_path) = file_entry.thumbnail_path else {
        return Err(not_found());
    };
    return Ok(ServeFile::new(data_path(&thumbnail_path))
        .oneshot(request)
        .await?);
}

pub async fn mkdir(
    State(appstate): State<AppState>,
    Path(file_path): Path<String>,
) -> ApiResult<()> {
    let (dir, name) = split_name(&file_path);

    info!("Creating dir {name} in {dir}");

    let (owner, owner_type) = match dir {
        "" => ("public".to_string(), "public".to_string()),
        _ => {
            let ownerlist = Ownerlist(&[("public", "public")]);

            let (parent_dir, parent_name) = split_name(dir);

            let dir_entry = entries::Entity::find()
                .filter(ownerlist.to_condition())
                .filter(entries::Column::Dir.eq(parent_dir))
                .filter(entries::Column::Name.eq(parent_name))
                .filter(entries::Column::IsDir.eq(true))
                .one(&appstate.db)
                .await?
                .ok_or_else(|| not_found())?;
            (dir_entry.owner, dir_entry.owner_type)
        }
    };

    entries::ActiveModel {
        owner: ActiveValue::Set(owner),
        dir: ActiveValue::Set(dir.to_owned()),
        name: ActiveValue::Set(name.to_owned()),
        owner_type: ActiveValue::Set(owner_type),
        is_dir: ActiveValue::Set(true),
        creator: ActiveValue::Set("todo".to_owned()),
        storage_path: ActiveValue::NotSet,
        thumbnail_path: ActiveValue::NotSet,
        created_at: ActiveValue::Set(chrono::Local::now().naive_local()),
    }
    .insert(&appstate.db)
    .await?;

    Ok(())
}

async fn delete_recursive(appstate: &AppState, path: String) -> ApiResult<()> {
    let (dir, name) = split_name(&path);

    let ownerlist = Ownerlist(&[("public", "public")]);

    let entry = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(ownerlist.to_condition())
        .one(&appstate.db)
        .await?
        .ok_or_else(|| not_found())?;

    let is_dir = entry.is_dir;
    if is_dir {
        // Delete children
        let child_entries = get_dir_entries(appstate, &path).await?;
        let results = futures::future::join_all(child_entries.into_iter().map(|e| async move {
            let appstate = appstate.clone();
            delete_recursive(&appstate, format!("{}/{}", e.dir, e.name)).await
        }))
        .await;
        for r in results {
            r?;
        }
    }

    let storage_path = entry.storage_path.clone();
    let thumbnail_path = entry.thumbnail_path.clone();

    entry.delete(&appstate.db).await?;

    if !is_dir {
        if let Some(storage_path) = storage_path {
            tokio::fs::remove_file(data_path(&storage_path)).await?;
        }
        if let Some(thumbnail_path) = thumbnail_path {
            tokio::fs::remove_file(data_path(&thumbnail_path)).await?;
        }
    }
    Ok(())
}

pub async fn delete(State(appstate): State<AppState>, Path(path): Path<String>) -> ApiResult<()> {
    delete_recursive(&appstate, path).await
}

const THUMBNAIL_MAX_WH: (u32, u32) = (120, 80);
pub async fn upload(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    body: Body,
) -> ApiResult<()> {
    let (dir, name) = split_name(&path);

    let file_ext = name.rfind(".").map(|u| &name[u..]).unwrap_or("");

    let storage_path = format!("{}{}", Uuid::now_v7(), file_ext);

    // TODO upsert
    let entry = entries::ActiveModel {
        owner: ActiveValue::Set("public".into()),
        dir: ActiveValue::Set(dir.into()),
        name: ActiveValue::Set(name.into()),
        owner_type: ActiveValue::Set("public".into()),
        is_dir: ActiveValue::Set(false),
        creator: ActiveValue::Set("todo".into()),
        storage_path: ActiveValue::Set(Some(storage_path.clone())),
        thumbnail_path: ActiveValue::NotSet,
        created_at: ActiveValue::Set(chrono::Local::now().naive_local()),
    }
    .insert(&appstate.db)
    .await?;

    let mut file = tokio::fs::File::create(data_path(&storage_path)).await?;

    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|_| axum_anyhow::bad_request("Bad request", "Failed to read request body"))?;

        file.write_all(&chunk).await.map_err(|err| {
            error!("Failed to write chunk to file at {dir}/{name}: {err:#?}");
            axum_anyhow::internal_error(
                "Failed to write chunk to file",
                "See logs for more details",
            )
        })?;
    }

    let thumbnail_path = format!("{storage_path}.thumbnail.png");
    let thumbnail_path_clone = thumbnail_path.clone();

    let mut thumbnailer = Thumbnailer::new(THUMBNAIL_MAX_WH.0, THUMBNAIL_MAX_WH.1);
    thumbnailer.allow_any_rotation();

    let thumb_res: anyhow::Result<()> = tokio::task::spawn_blocking(move || {
        let thumb = thumbnailer.get(data_path(&storage_path))?;
        info!("Saving thumbnail to {}", data_path(&thumbnail_path_clone));
        thumb.save(data_path(&thumbnail_path_clone))?;

        Ok(())
    })
    .await
    .unwrap();

    match thumb_res {
        Ok(_) => {
            let mut model: entries::ActiveModel = entry.into();
            model.thumbnail_path = Set(Some(thumbnail_path));
            entries::Entity::update(model).exec(&appstate.db).await?;
        }
        Err(err) => {
            info!("Failed to create or save thumbnail for {dir}/{name} with {err}");
        }
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct RenameRequest {
    new_name: String,
}

pub async fn rename(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    Json(RenameRequest { new_name }): Json<RenameRequest>,
) -> ApiResult<()> {
    if new_name.contains('/') {
        return Err(axum_anyhow::bad_request(
            "Invalid filename",
            "New filename cannot contain '/'",
        ));
    }

    info!("Renaming {path} to {new_name}");

    let (dir, name) = split_name(&path);

    let ownerlist = Ownerlist(&[("public", "public")]);

    let entry = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(ownerlist.to_condition())
        .one(&appstate.db)
        .await?
        .ok_or_else(|| not_found())?;

    let active_model = entries::ActiveModel {
        name: ActiveValue::Set(new_name),
        ..Default::default()
    };

    entries::Entity::update_many()
        .set(active_model)
        .filter(entries::Column::Dir.eq(entry.dir))
        .filter(entries::Column::Name.eq(entry.name))
        .filter(entries::Column::Owner.eq(entry.owner))
        .exec(&appstate.db)
        .await?;

    Ok(())
}
