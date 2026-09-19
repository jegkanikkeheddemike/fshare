use std::{fs::Metadata, path::PathBuf};

use axum::{
    Json,
    body::Body,
    extract::{Path, Request},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};
use axum_anyhow::ApiResult;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
// use tokio_util::io::ReaderStream;

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

    println!("Resulting path: {path:#?}");

    let Ok(canon_path) = tokio::fs::canonicalize(&path).await else {
        return not_found();
    };
    println!("Canon path: {path:#?}");
    if !canon_path.starts_with("/public/") {
        return not_found();
    }

    let md = match tokio::fs::metadata(&canon_path).await {
        Ok(md) => md,
        Err(err) => {
            eprintln!("WARNING: Failed to read metadata of: {canon_path:?} with error: {err:#?}");
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
            eprintln!("WARNING: Failed to read dir at: {canon_path:?} with error: {err:#?}");
            return not_found();
        }
    };
    let mut entries = vec![];
    while let Ok(Some(entry)) = dir.next_entry().await {
        let Some(filename) = entry.file_name().to_str().map(|s| s.to_string()) else {
            println!(
                "WARNING: Failed to convert filename to valid string: {:#?}",
                entry.file_name()
            );
            continue;
        };
        let Ok(e_md) = tokio::fs::metadata(entry.path().as_path()).await else {
            eprintln!("WARNING: Failed to read metadata at {:?}", entry.path());
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

    return Ok(Json(DirInfo { entries }));
}
