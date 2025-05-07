use std::sync::Arc;

use sqlx::{Database, MySql, Pool, Postgres, Sqlite};

use super::{ConnectionPool, DatabaseType};

pub struct DBConnectionOptions {
    pub connection_string: String,
}

impl Default for DBConnectionOptions {
    fn default() -> Self {
        Self {
            connection_string: "".to_string(),
        }
    }
}

pub struct DBConnection {
    pub(crate) options: DBConnectionOptions,
    pub pool: tokio::sync::OnceCell<Option<Arc<ConnectionPool>>>,
}

/// Trait for database operations
#[tower_lsp::async_trait]
pub trait DatabaseOperations: Send + Sync {
    async fn execute_query(&self, query: &str) -> anyhow::Result<(serde_json::Value, usize)>;
    async fn get_tables(&self) -> anyhow::Result<Vec<String>>;
    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<String>>;
    async fn check_connection(&self) -> anyhow::Result<bool>;
}

/// Database connection manager
pub struct DBSet<DB>
where
    DB: Database,
{
    pool: Arc<Pool<DB>>,
}

impl<DB> DBSet<DB>
where
    DB: Database,
{
    pub fn new(pool: Pool<DB>) -> Self {
        DBSet {
            pool: Arc::new(pool),
        }
    }

    pub fn pool(&self) -> Arc<Pool<DB>> {
        Arc::clone(&self.pool)
    }
}

#[tower_lsp::async_trait]
pub trait DatabaseManager<DB>
where
    DB: Database,
{
    async fn create(options: &DBConnectionOptions) -> anyhow::Result<DBSet<DB>>;
}

impl DBConnection {
    async fn from_options(options: &DBConnectionOptions) -> anyhow::Result<ConnectionPool> {
        let connection_string = &options.connection_string;
        // Parse the connection string to determine database type
        let db_type = if connection_string.starts_with("sqlite:") {
            DatabaseType::SQLite
        } else if connection_string.starts_with("mysql:") || connection_string.contains("mysql://")
        {
            DatabaseType::MySQL
        } else if connection_string.starts_with("postgres:")
            || connection_string.contains("postgresql://")
        {
            DatabaseType::PostgreSQL
        } else {
            return Err(anyhow::anyhow!(
                "Unsupported database type in connection string"
            ));
        };

        match db_type {
            DatabaseType::SQLite => {
                let db_set = DBSet::<Sqlite>::create(options).await?;
                return Ok(db_set.into());
            }
            DatabaseType::MySQL => {
                let db_set = DBSet::<MySql>::create(options).await?;
                return Ok(db_set.into());
            }
            DatabaseType::PostgreSQL => {
                let db_set = DBSet::<Postgres>::create(options).await?;
                return Ok(db_set.into());
            }
        }
    }

    pub async fn get_pool(&self) -> Option<Arc<ConnectionPool>> {
        self.pool
            .get_or_init(|| async {
                match Self::from_options(&self.options).await {
                    Ok(pool) => Some(Arc::new(pool)),
                    Err(_) => None,
                }
            })
            .await
            .clone()
    }
}
