use axum::{
    Extension, Json,
    body::Body,
    extract::{Path, Query, Request, State},
    response::IntoResponse,
};
use axum_anyhow::{ApiError, ApiResult};
use axum_keycloak_auth::{
    KeycloakAuthStatus,
    decode::{KeycloakToken, ProfileAndEmail},
};
use futures::StreamExt;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{self, Set},
    ColumnTrait, Condition, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
    sqlx::types::chrono,
};
use thumbnails::Thumbnailer;
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{error, info};
use uuid::Uuid;

use crate::{AppState, access_key_utils, db::entries, not_found};

#[derive(Debug, serde::Serialize)]
pub struct EntryInfo {
    relative_name: String,
    is_dir: bool,
    mime: Option<String>,
    access_key: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct DirInfo {
    entries: Vec<EntryInfo>,
}

pub async fn get_root_dir(
    appstate: State<AppState>,
    Extension(auth): Extension<AuthStatus>,
) -> ApiResult<Json<DirInfo>> {
    let Json(mut dir) = get_dir(appstate, Path("".into()), Extension(auth.clone())).await?;

    if let KeycloakAuthStatus::Success(_) = auth {
        dir.entries.push(EntryInfo {
            relative_name: "My Files".to_string(),
            is_dir: true,
            mime: None,
            access_key: None,
        });
    }

    return Ok(Json(dir));
}

const OWNERTYPE_USER: &'static str = "user";
const OWNERTYPE_PUBLIC: &'static str = "public";
#[allow(unused)]
const OWNERTYPE_OIDC_GROUP: &'static str = "oidc_group";
#[allow(unused)]
const OWNERTYPE_SHARED_DIR: &'static str = "shared_dir";

fn get_ownerlist(auth: &AuthStatus) -> Condition {
    let public_cond = Condition::all()
        .add(entries::Column::Owner.eq(OWNERTYPE_PUBLIC))
        .add(entries::Column::OwnerType.eq(OWNERTYPE_PUBLIC));

    if let KeycloakAuthStatus::Success(auth) = auth {
        let user_cond = Condition::all()
            .add(entries::Column::Owner.eq(&auth.subject))
            .add(entries::Column::OwnerType.eq(OWNERTYPE_USER));

        return Condition::any().add(public_cond).add(user_cond);
    }

    public_cond
}

fn split_name<'a>(path: &'a str) -> (&'a str, &'a str) {
    match path.rsplit_once("/") {
        Some((dir, file)) => (dir, file),
        None => {
            // In root dir
            ("", path.trim_end_matches('/'))
        }
    }
}

fn data_path(path: &str) -> String {
    format!("/data/{path}")
}

async fn get_dir_entries(
    appstate: &AppState,
    dir_name: &str,
    auth: &AuthStatus,
) -> ApiResult<Vec<entries::Model>> {
    entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir_name))
        .filter(get_ownerlist(auth))
        .order_by_asc(entries::Column::Name)
        .all(&appstate.db)
        .await
        .map_err(|err| axum_anyhow::internal_error("Internal error", &err.to_string()))
}

pub async fn get_dir(
    State(appstate): State<AppState>,
    Path(dir_name): Path<String>,
    Extension(auth): Extension<AuthStatus>,
) -> ApiResult<Json<DirInfo>> {
    let dir_name = dir_name.trim_matches('/');
    info!("Dir lookup in {dir_name}");

    let db_entries = get_dir_entries(&appstate, dir_name, &auth).await?;

    let db = &appstate.db;

    let dir: Vec<ApiResult<_>> =
        futures::future::join_all(db_entries.into_iter().map(|e| async move {
            let (mime, access_key) = if e.is_dir {
                (None, None)
            } else {
                let mime = mime_guess::from_path(&e.name)
                    .first()
                    .map(|m| m.essence_str().to_owned());

                if e.owner_type != OWNERTYPE_PUBLIC {
                    // Create access keuy
                    let key = access_key_utils::get_or_create(&e.reference_id, None, db).await?;
                    (mime, Some(key))
                } else {
                    (mime, None)
                }
            };

            Ok(EntryInfo {
                relative_name: e.name,
                is_dir: e.is_dir,
                mime,
                access_key,
            })
        }))
        .await;

    let dir = dir.into_iter().collect::<ApiResult<Vec<_>>>()?;

    return Ok(Json(DirInfo { entries: dir }));
}

#[derive(Debug, serde::Deserialize)]
pub struct MaybeAccessKey {
    access_key: Option<String>,
}

pub async fn get_file(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    Query(maybe_access_key): Query<MaybeAccessKey>,
    Extension(auth): Extension<AuthStatus>,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (dir, name) = split_name(&path);

    info!("File lookup at {dir} {name}");

    let file_entry = match maybe_access_key.access_key {
        Some(access_key) => {
            access_key_utils::get_entry_from_key(&path, &access_key, &appstate.db).await?
        }
        None => {
            // Lookup with personal rights
            entries::Entity::find()
                .filter(entries::Column::Dir.eq(dir))
                .filter(entries::Column::Name.eq(name))
                .filter(get_ownerlist(&auth))
                .one(&appstate.db)
                .await?
                .ok_or_else(|| not_found())?
        }
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
    Extension(auth): Extension<AuthStatus>,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (dir, name) = split_name(&path);

    let file = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(get_ownerlist(&auth))
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

fn assert_logged_in(auth: &AuthStatus) -> ApiResult<&KeycloakToken<String, ProfileAndEmail>> {
    match auth {
        KeycloakAuthStatus::Success(keycloak_token) => Ok(keycloak_token),
        KeycloakAuthStatus::Failure(_) => Err(axum_anyhow::forbidden("Forbidden", "Not logged in")),
    }
}

type AuthStatus = KeycloakAuthStatus<String, ProfileAndEmail>;

async fn get_dir_info(
    path: &str,
    appstate: &AppState,
    auth: &AuthStatus,
) -> ApiResult<(String, String)> {
    match path {
        "" => Ok((OWNERTYPE_PUBLIC.to_string(), OWNERTYPE_PUBLIC.to_string())),
        "My Files" => {
            let token = assert_logged_in(auth)?;
            Ok((token.subject.clone(), OWNERTYPE_USER.to_string()))
        }
        _ => {
            let (parent_dir, parent_name) = split_name(path);

            let dir_entry = entries::Entity::find()
                .filter(get_ownerlist(&auth))
                .filter(entries::Column::Dir.eq(parent_dir))
                .filter(entries::Column::Name.eq(parent_name))
                .filter(entries::Column::IsDir.eq(true))
                .one(&appstate.db)
                .await?
                .ok_or_else(|| not_found())?;
            Ok((dir_entry.owner, dir_entry.owner_type))
        }
    }
}

pub async fn mkdir(
    State(appstate): State<AppState>,
    Path(file_path): Path<String>,
    Extension(auth): Extension<AuthStatus>,
) -> ApiResult<()> {
    let (dir, name) = split_name(&file_path);

    let token = assert_logged_in(&auth)?;
    let (owner, owner_type) = get_dir_info(dir, &appstate, &auth).await?;

    info!("Creating dir {name} in {dir}");

    entries::ActiveModel {
        owner: ActiveValue::Set(owner),
        dir: ActiveValue::Set(dir.to_owned()),
        name: ActiveValue::Set(name.to_owned()),
        owner_type: ActiveValue::Set(owner_type),
        is_dir: ActiveValue::Set(true),
        creator: ActiveValue::Set(token.subject.clone()),
        storage_path: ActiveValue::NotSet,
        thumbnail_path: ActiveValue::NotSet,
        created_at: ActiveValue::Set(chrono::Local::now().naive_local()),
        reference_id: ActiveValue::Set(Uuid::now_v7().to_string()),
    }
    .insert(&appstate.db)
    .await?;

    Ok(())
}

async fn delete_recursive(appstate: &AppState, path: String, auth: &AuthStatus) -> ApiResult<()> {
    let (dir, name) = split_name(&path);

    let entry = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(get_ownerlist(&auth))
        .one(&appstate.db)
        .await?
        .ok_or_else(|| not_found())?;

    let is_dir = entry.is_dir;
    if is_dir {
        // Delete children
        let child_entries = get_dir_entries(appstate, &path, auth).await?;
        let results = futures::future::join_all(child_entries.into_iter().map(|e| async move {
            delete_recursive(&appstate, format!("{}/{}", e.dir, e.name), auth).await
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

pub async fn delete(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    Extension(auth): Extension<AuthStatus>,
) -> ApiResult<()> {
    delete_recursive(&appstate, path, &auth).await
}

const THUMBNAIL_MAX_WH: (u32, u32) = (120, 80);
pub async fn upload(
    State(appstate): State<AppState>,
    Path(path): Path<String>,
    Extension(auth): Extension<AuthStatus>,
    body: Body,
) -> ApiResult<()> {
    let (dir, name) = split_name(&path);
    let token = assert_logged_in(&auth)?;
    let (owner, owner_type) = get_dir_info(dir, &appstate, &auth).await?;

    let file_ext = name.rfind(".").map(|u| &name[u..]).unwrap_or("");

    let storage_path = format!("{}{}", Uuid::now_v7(), file_ext);

    // TODO: upsert?
    let entry = entries::ActiveModel {
        owner: ActiveValue::Set(owner),
        dir: ActiveValue::Set(dir.into()),
        name: ActiveValue::Set(name.into()),
        owner_type: ActiveValue::Set(owner_type),
        is_dir: ActiveValue::Set(false),
        creator: ActiveValue::Set(token.subject.clone()),
        storage_path: ActiveValue::Set(Some(storage_path.clone())),
        thumbnail_path: ActiveValue::NotSet,
        created_at: ActiveValue::Set(chrono::Local::now().naive_local()),
        reference_id: ActiveValue::Set(Uuid::now_v7().to_string()),
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
    Extension(auth): Extension<AuthStatus>,
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

    let entry = entries::Entity::find()
        .filter(entries::Column::Dir.eq(dir))
        .filter(entries::Column::Name.eq(name))
        .filter(get_ownerlist(&auth))
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
