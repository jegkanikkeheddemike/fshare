use std::{ops::Deref, sync::LazyLock};

use redis::aio::MultiplexedConnection;
use tokio::sync::OnceCell;

static CLIENT: LazyLock<redis::Client> =
    LazyLock::new(|| redis::Client::open("redis://redis/").unwrap());

static CONNECTION: OnceCell<MultiplexedConnection> = OnceCell::const_new();

pub async fn init() -> anyhow::Result<()> {
    let client = CLIENT.deref();

    let conn = client.get_multiplexed_async_connection().await.unwrap();
    CONNECTION.set(conn)?;

    Ok(())
}

pub async fn get() -> MultiplexedConnection {
    CONNECTION.get().unwrap().clone()
}
