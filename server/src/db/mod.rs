use std::{collections::HashMap, sync::Arc};

use connection::{DBConnection, DBConnectionOptions, DatabaseOperations};
use tokio::sync::RwLock;

pub mod connection;
mod mysql;
mod postgres;
mod sqlite;

static DB_POOL_MAP: once_cell::sync::Lazy<RwLock<HashMap<String, Arc<DBConnection>>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

pub type ConnectionPool = Box<dyn DatabaseOperations + Send + Sync>;

/// Supported database types
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    SQLite,
    MySQL,
    PostgreSQL,
    // Add more as needed
}

pub async fn from_cache(id: &str, option: DBConnectionOptions) -> Arc<DBConnection> {
    {
        let map = DB_POOL_MAP.read().await;
        let v = map.get(id);
        if let Some(v) = v {
            return Arc::clone(v);
        }
    }

    {
        let db_connection = DBConnection {
            options: option,
            pool: tokio::sync::OnceCell::new(),
        };
        DB_POOL_MAP
            .write()
            .await
            .insert(id.to_string(), Arc::new(db_connection));
    }
    Arc::clone(DB_POOL_MAP.read().await.get(id).unwrap())
}
