use std::{fs::Metadata, path::PathBuf};

use axum::{Json, extract::Path, response::IntoResponse};
use axum_anyhow::ApiResult;
use tokio::fs::{self};
use tracing::{error, warn};

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

pub async fn upload(Path(path): Path<PathBuf>) -> ApiResult<()> {
    Ok(())
}
