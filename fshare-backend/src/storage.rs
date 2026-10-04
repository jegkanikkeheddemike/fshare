use std::hash::{DefaultHasher, Hash, Hasher};
use std::str::FromStr;
use std::{fs::Metadata, path::PathBuf};

use axum::Extension;
use axum::extract::Request;
use axum::http::uri::PathAndQuery;
use axum::http::{Uri, uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, body::Body, extract::Path};
use axum_anyhow::ApiResult;
use axum_keycloak_auth::KeycloakAuthStatus;
use axum_keycloak_auth::decode::ProfileAndEmail;
use futures_util::StreamExt;
use thumbnails::Thumbnailer;
use tokio::fs::{self};
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};
use uuid::Uuid;

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

async fn get_md(
    path: PathBuf,
    auth_status: &KeycloakAuthStatus<String, ProfileAndEmail>,
) -> ApiResult<(Metadata, PathBuf)> {
    let canon_path = match path.starts_with("My Files") {
        true => {
            if let KeycloakAuthStatus::Success(token) = auth_status {
                let user_dir = PathBuf::from("/users").join(&token.subject);

                // Create user dir if it does not exist
                tokio::fs::create_dir_all(&user_dir).await.map_err(|err| {
                    error!(
                        "Failed to create user directory at {user_dir:?} with err: {:#?}",
                        err
                    );
                    return axum_anyhow::internal_error(
                        "Failed to create user directory",
                        "See logs for more details",
                    );
                })?;
                let full_path = user_dir.join(path.components().skip(1).collect::<PathBuf>());
                let Ok(canon_path) = tokio::fs::canonicalize(&full_path).await else {
                    return not_found();
                };

                if !full_path.starts_with(format!("/users/{}", &token.subject)) {
                    return not_found();
                }

                canon_path
            } else {
                return not_found();
            }
        }
        false => {
            let path = PathBuf::from("/public").join(path);
            let Ok(canon_path) = tokio::fs::canonicalize(&path).await else {
                return not_found();
            };
            if !canon_path.starts_with("/public/") {
                return not_found();
            }
            canon_path
        }
    };

    let md = match tokio::fs::metadata(&canon_path).await {
        Ok(md) => md,
        Err(err) => {
            warn!("Failed to read metadata of: {canon_path:?} with error: {err:#?}");
            return not_found();
        }
    };
    return Ok((md, canon_path));
}

pub async fn prepare_file_req(
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = PathBuf::from(
        urlencoding::decode(&request.uri().to_string()[1..])
            .unwrap()
            .to_string(),
    );
    let (_file_md, canon_path) = match get_md(path.clone(), &auth_status).await {
        Ok(r) => r,
        Err(err) => return err.into_response(),
    };

    let mut parts = uri::Parts::default();
    parts.path_and_query = Some(PathAndQuery::from_str(canon_path.to_str().unwrap()).unwrap());
    *request.uri_mut() = Uri::from_parts(parts).unwrap();

    next.run(request).await
}

pub async fn prepare_thumbnail(
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = PathBuf::from(
        urlencoding::decode(&request.uri().to_string()[1..])
            .unwrap()
            .to_string(),
    );
    let (_file_md, file_canon_path) = match get_md(path.clone(), &auth_status).await {
        Ok(r) => r,
        Err(err) => return err.into_response(),
    };
    let mut hasher = DefaultHasher::new();
    file_canon_path.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());

    let thumbnail_path = PathBuf::from("/thumbnails")
        .join(&hash)
        .with_extension("png");

    match tokio::fs::try_exists(&thumbnail_path).await {
        Ok(false) => {
            let file_canon_path_clone = file_canon_path.clone();
            let thumbnail_path_clone = thumbnail_path.clone();
            if let Err(err) = tokio::task::spawn_blocking(move || {
                let thumbnailer = Thumbnailer::new(120, 80);
                match thumbnailer.get(&file_canon_path_clone) {
                    Ok(thumbnail) => {

                        thumbnail.save(&thumbnail_path_clone).unwrap();
                        info!("Generated thumbnail for {file_canon_path_clone:#?} at {thumbnail_path_clone:#?}");
                    }
                    Err(err) => {
                        let err_str = err.to_string();
                        if !err_str.starts_with("Unsupported MIME type") {
                            info!("Failed to generate thumbnail for {file_canon_path_clone:#?} with: {err:#?}");
                        }
                    }
                };
            })
            .await
            {
                error!(
                    "Failed to generate and save thumbnail for {file_canon_path:#?} with err: {err:#?}"
                );
                return axum_anyhow::internal_error(
                    "Failed to generate thumbnail",
                    "See logs for more details",
                )
                .into_response();
            }
        }
        Ok(true) => {
            // TODO: Check if outdated
        }
        Err(err) => {
            error!("Failed to check thumbnail dir at {thumbnail_path:#?} with err: {err:#?}");
            return axum_anyhow::internal_error(
                "Failed to lookup thumbnail",
                "See logs for more details.",
            )
            .into_response();
        }
    };

    let mut parts = uri::Parts::default();
    parts.path_and_query = Some(PathAndQuery::from_str(&format!("/{hash}.png")).unwrap());
    *request.uri_mut() = Uri::from_parts(parts).unwrap();

    next.run(request).await
}

pub async fn get_root_dir(
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
) -> ApiResult<Json<DirInfo>> {
    let mut public_files = get_dir(Path(PathBuf::from("")), Extension(auth_status.clone())).await?;

    if let KeycloakAuthStatus::Success(_) = auth_status {
        public_files.entries.push(EntryInfo {
            relative_name: "My Files".to_string(),
            is_dir: true,
            mime: None,
            access_key: None,
        });
    }

    return Ok(public_files);
}

pub async fn get_dir(
    Path(path): Path<PathBuf>,
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
) -> ApiResult<Json<DirInfo>> {
    let (md, canon_path) = get_md(path, &auth_status).await?;
    if !md.is_dir() {
        return not_found();
    }
    let mut dir = match fs::read_dir(canon_path.as_path()).await {
        Ok(dir) => dir,
        Err(err) => {
            warn!("Failed to read dir at: {canon_path:?} with error: {err:#?}");
            return not_found();
        }
    };
    let mut entries = vec![];
    while let Ok(Some(entry)) = dir.next_entry().await {
        let Some(filename) = entry.file_name().to_str().map(|s| s.to_string()) else {
            warn!(
                "Failed to convert filename to valid string: {:#?}",
                entry.file_name()
            );
            continue;
        };
        let Ok(e_md) = tokio::fs::metadata(entry.path().as_path()).await else {
            warn!("Failed to read metadata at {:?}", entry.path());
            continue;
        };
        if e_md.is_symlink() {
            // Ignore symlinks
            continue;
        }
        entries.push(EntryInfo {
            is_dir: e_md.is_dir(),
            mime: mime_guess::from_path(&filename)
                .first()
                .map(|m| m.to_string()),
            relative_name: filename,
            access_key: None,
        });
    }

    entries.sort_by(|a, b| a.relative_name.cmp(&b.relative_name));
    entries.sort_by_key(|e| !e.is_dir);

    return Ok(Json(DirInfo { entries }));
}

pub async fn mkdir(
    Path(path): Path<PathBuf>,
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
) -> ApiResult<()> {
    let parent_dir = {
        let mut t = path.clone();
        t.pop();
        t
    };
    let (md, mut canon_path) = get_md(parent_dir, &auth_status).await?;
    if !md.is_dir() {
        return Err(axum_anyhow::bad_request(
            "Invalid path",
            "Parent path is not a directory",
        ));
    }

    canon_path.push(path.file_name().unwrap());

    if let Err(err) = tokio::fs::create_dir(&canon_path).await {
        error!(
            "Failed to create dir at {canon_path:#?} with err: {:#?}",
            err
        );

        return Err(axum_anyhow::internal_error(
            "Failed to create dir.",
            "See logs for more datails",
        ));
    }

    Ok(())
}

pub async fn delete(
    Path(path): Path<PathBuf>,
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
) -> ApiResult<()> {
    let (md, canon_path) = get_md(path, &auth_status).await?;
    if md.is_dir() {
        if let Err(err) = tokio::fs::remove_dir_all(&canon_path).await {
            error!("Failed to delete directory at {canon_path:#?}: {err:#?}");
            return Err(axum_anyhow::internal_error(
                "Failed to remove dir.",
                "See logs",
            ));
        }
    } else {
        if let Err(err) = tokio::fs::remove_file(&canon_path).await {
            error!("Failed to delete file at {canon_path:#?}: {err:#?}");
            return Err(axum_anyhow::internal_error(
                "Failed to remove file.",
                "See logs",
            ));
        }
    }

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct RenameRequest {
    new_name: String,
}

pub async fn rename(
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
    Path(path): Path<PathBuf>,
    Json(rename_req): Json<RenameRequest>,
) -> ApiResult<()> {
    let (_md, canon_path) = get_md(path, &auth_status).await?;

    let new_path = canon_path.parent().unwrap().join(rename_req.new_name);

    match get_md(new_path.clone(), &auth_status).await {
        Ok((md, _)) => {
            if !md.is_dir() {
                return Err(axum_anyhow::bad_request(
                    "Failed to rename item.",
                    "File already exists at destination path.",
                ));
            }
        }
        Err(_) => {
            //Destination does not exist, we can proceed with the rename
        }
    };

    tokio::fs::rename(&canon_path, new_path)
        .await
        .map_err(|err| {
            info!(
                "Failed to rename item at {canon_path:#?} with err: {:#?}",
                err
            );
            return axum_anyhow::internal_error("Failed to rename item.", &err.to_string());
        })?;

    Ok(())
}

pub async fn upload(
    Path(path): Path<PathBuf>,
    Extension(auth_status): Extension<KeycloakAuthStatus<String, ProfileAndEmail>>,
    body: Body,
) -> ApiResult<()> {
    let parent_dir = {
        let mut t = path.clone();
        t.pop();
        t
    };
    let (md, mut canon_path) = get_md(parent_dir, &auth_status).await?;
    if !md.is_dir() {
        return Err(axum_anyhow::bad_request(
            "Invalid path",
            "Parent path is not a directory",
        ));
    }

    canon_path.push(path.file_name().unwrap());

    let mut file = match tokio::fs::File::create(&canon_path).await {
        Ok(file) => file,
        Err(err) => {
            error!(
                "Failed to create file at {canon_path:#?} with err: {:#?}",
                err
            );
            return Err(axum_anyhow::internal_error(
                "Failed to create file",
                "See logs for more details",
            ));
        }
    };
    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| {
            axum_anyhow::internal_error(
                "Failed to read chunk from request body",
                "Perhaps fix the request.",
            )
        })?;

        file.write_all(&chunk).await.map_err(|err| {
            error!("Failed to write chunk to file at {canon_path:#?}: {err:#?}");
            axum_anyhow::internal_error(
                "Failed to write chunk to file",
                "See logs for more details",
            )
        })?;
    }

    Ok(())
}
