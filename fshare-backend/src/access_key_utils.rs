use std::{ops::Deref, time::Duration};

use axum_anyhow::ApiResult;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    sqlx::types::chrono,
};
use tracing::info;
use uuid::Uuid;

use crate::{
    db::{access_keys, entries},
    not_found,
};

const DEFAULT_ACCESS_KEY_DURATION: Duration = Duration::from_hours(12);
const DEFAULT_MIN_REM_KEY_DURATION: Duration = Duration::from_hours(6);

pub async fn get_or_create(
    reference_id: &str,
    expiration: Option<Duration>,
    db: &DatabaseConnection,
) -> ApiResult<String> {
    let existing_key = access_keys::Entity::find()
        .filter(access_keys::Column::EntryReference.eq(reference_id))
        .filter(access_keys::Column::Retracted.eq(false))
        .filter(
            access_keys::Column::ValidUntil
                .gt((chrono::Local::now() + DEFAULT_MIN_REM_KEY_DURATION).date_naive()),
        )
        .order_by_desc(access_keys::Column::ValidUntil)
        .one(db)
        .await?;

    if let Some(key) = existing_key {
        info!(
            "Found existing key with rem time {}",
            (key.valid_until - chrono::Local::now().naive_local())
        );
        return Ok(key.key);
    }

    let access_key = Uuid::new_v4().to_string();

    let expiration = chrono::Local::now() + expiration.unwrap_or(DEFAULT_ACCESS_KEY_DURATION);

    access_keys::Entity::insert(access_keys::ActiveModel {
        key: ActiveValue::Set(access_key.clone()),
        entry_reference: ActiveValue::Set(reference_id.to_string()),
        created_at: ActiveValue::Set(chrono::Local::now().naive_local()),
        valid_until: ActiveValue::Set(expiration.naive_local()),
        retracted: ActiveValue::Set(false),
    })
    .exec(db)
    .await?;
    Ok(access_key)
}

pub async fn get_entry_from_key(
    path: &str,
    access_key: &str,
    db: &DatabaseConnection,
) -> ApiResult<entries::Model> {
    let key = access_keys::Entity::find_by_id(access_key)
        .require_one(db)
        .await
        .map_err(|_| not_found())?;

    if key.retracted || chrono::Local::now().naive_local() > key.valid_until {
        return Err(not_found());
    }

    let entry = entries::Entity::find()
        .filter(entries::Column::ReferenceId.eq(key.entry_reference))
        .require_one(db)
        .await
        .map_err(|_| not_found())?;

    let entry_path = match entry.dir.deref() {
        "" => entry.name.clone(),
        _ => format!("{}/{}", &entry.dir, &entry.name),
    };
    if entry_path != path {
        return Err(not_found());
    }

    Ok(entry)
}
