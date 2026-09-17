use std::path::PathBuf;

use axum::{Json, body::Body, extract::Path};
use axum_anyhow::ApiResult;
use tokio::fs::{self, File};

#[derive(Debug, serde::Serialize)]
pub enum StorageLookup {
    Directory(Vec<String>),
    File(String),
}

pub async fn get_from_storage(Path(arg_path): Path<PathBuf>) -> ApiResult<Json<StorageLookup>> {
    println!("Received from call: {arg_path:?}");

    let path = PathBuf::from("public").join(arg_path);

    let not_found = Err(axum_anyhow::not_found(
        "Not found",
        "No item or directory at this path",
    ));

    let Ok(canon_path) = path.canonicalize() else {
        return not_found;
    };
    if !canon_path.starts_with("public/") {
        return not_found;
    }

    let md = match canon_path.metadata() {
        Ok(md) => md,
        Err(err) => {
            eprintln!("WARNING: Failed to read metadata of: {canon_path:?} with error: {err:#?}");
            return not_found;
        }
    };
    if md.is_dir() {
        let mut dir = match fs::read_dir(canon_path.as_path()).await {
            Ok(dir) => dir,
            Err(err) => {
                eprintln!("WARNING: Failed to read dir at: {canon_path:?} with error: {err:#?}");
                return not_found;
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
            entries.push(filename);
        }

        return Ok(Json(StorageLookup::Directory(entries)));
    } else if md.is_file() {
        let f = match File::open(canon_path.as_path()).await {
            Ok(f) => f,
            Err(err) => {
                eprintln!("WARNING: Failed to read file at: {canon_path:?} with error: {err:#?}");
                return not_found;
            }
        };
        // let Some(filename) = canon_path
        //     .file_name()
        //     .expect("Will always have a filename")
        //     .to_str()
        //     .map(|s| s.to_string())
        // else {
        //     println!(
        //         "WARNING: Failed to convert filename to valid string: {:#?}",
        //         canon_path.file_name()
        //     );
        //     return not_found;
        // };
        let mime = mime_guess::from_path(canon_path.as_path()).first_or_octet_stream();

        todo!()
    } else {
        return Err(axum_anyhow::forbidden(
            "Symlink found",
            "Following symlinks is currently not permitted",
        ));
    }
}
