use std::{fs::Metadata, path::PathBuf};

use axum::{Json, body::Body, extract::Path, response::IntoResponse};
use axum_anyhow::ApiResult;
use futures_util::StreamExt;
use tokio::fs::{self};
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};

#[derive(Debug, serde::Serialize)]
pub struct EntryInfo {
    relative_name: String,
    is_dir: bool,
    mime: Option<String>,
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

async fn get_md(path: PathBuf) -> ApiResult<(Metadata, PathBuf)> {
    let path = PathBuf::from("/public").join(path);

    let Ok(canon_path) = tokio::fs::canonicalize(&path).await else {
        return not_found();
    };
    if !canon_path.starts_with("/public/") {
        return not_found();
    }

    let md = match tokio::fs::metadata(&canon_path).await {
        Ok(md) => md,
        Err(err) => {
            warn!("Failed to read metadata of: {canon_path:?} with error: {err:#?}");
            return not_found();
        }
    };
    return Ok((md, canon_path));
}

pub async fn get_root_dir() -> impl IntoResponse {
    return get_dir(Path(PathBuf::from(""))).await;
}

pub async fn get_dir(Path(path): Path<PathBuf>) -> ApiResult<Json<DirInfo>> {
    let (md, canon_path) = get_md(path).await?;
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
        });
    }

    entries.sort_by(|a, b| a.relative_name.cmp(&b.relative_name));
    entries.sort_by_key(|e| !e.is_dir);

    return Ok(Json(DirInfo { entries }));
}

pub async fn mkdir(Path(path): Path<PathBuf>) -> ApiResult<()> {
    let parent_dir = {
        let mut t = path.clone();
        t.pop();
        t
    };
    let (md, mut canon_path) = get_md(parent_dir).await?;
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

pub async fn delete(Path(path): Path<PathBuf>) -> ApiResult<()> {
    let (md, canon_path) = get_md(path).await?;
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
    Path(path): Path<PathBuf>,
    Json(rename_req): Json<RenameRequest>,
) -> ApiResult<()> {
    let (_md, canon_path) = get_md(path).await?;

    let new_path = canon_path.parent().unwrap().join(rename_req.new_name);

    match get_md(new_path.clone()).await {
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

pub async fn upload(Path(path): Path<PathBuf>, body: Body) -> ApiResult<()> {
    let parent_dir = {
        let mut t = path.clone();
        t.pop();
        t
    };
    let (md, mut canon_path) = get_md(parent_dir).await?;
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
