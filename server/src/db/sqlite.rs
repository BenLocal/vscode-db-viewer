use std::time::Duration;

use sqlx::{Column, Row, Sqlite, sqlite::SqlitePoolOptions};

use super::{
    ConnectionPool,
    connection::{DBConnectionOptions, DBSet, DatabaseManager, DatabaseOperations},
};

#[tower_lsp::async_trait]
impl DatabaseManager<Sqlite> for DBSet<Sqlite> {
    async fn create(options: &DBConnectionOptions) -> anyhow::Result<DBSet<Sqlite>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect_lazy(&options.connection_string)?;

        Ok(DBSet::new(pool))
    }
}

impl Into<ConnectionPool> for DBSet<Sqlite> {
    fn into(self) -> ConnectionPool {
        Box::new(SQLiteOperations(self))
    }
}

/// SQLite specific operations
pub struct SQLiteOperations(DBSet<Sqlite>);

#[tower_lsp::async_trait]
impl DatabaseOperations for SQLiteOperations {
    async fn execute_query(&self, query: &str) -> anyhow::Result<(serde_json::Value, usize)> {
        // For SELECT queries, fetch rows
        if query.trim().to_lowercase().starts_with("select") {
            let rows = sqlx::query(query).fetch_all(self.0.pool().as_ref()).await?;
            let total = rows.len();
            // Convert to JSON
            let mut result = Vec::new();
            for row in rows {
                let mut obj = serde_json::Map::new();

                // Convert each column to a JSON value
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name();
                    let value: Option<String> = row.try_get(i)?;
                    obj.insert(
                        column_name.to_string(),
                        serde_json::Value::String(value.unwrap_or_default()),
                    );
                }

                result.push(serde_json::Value::Object(obj));
            }

            Ok((serde_json::Value::Array(result), total))
        } else {
            // For non-SELECT queries, return affected rows
            let result = sqlx::query(query).execute(self.0.pool().as_ref()).await?;

            Ok((serde_json::Value::Null, result.rows_affected() as usize))
        }
    }

    async fn get_tables(&self) -> anyhow::Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(self.0.pool().as_ref())
        .await?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.try_get("name")?;
            tables.push(table_name);
        }

        Ok(tables)
    }

    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<String>> {
        let query = format!("PRAGMA table_info({})", table_name);
        let rows = sqlx::query(&query)
            .fetch_all(self.0.pool().as_ref())
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let column_name: String = row.try_get("name")?;
            columns.push(column_name);
        }

        Ok(columns)
    }

    async fn check_connection(&self) -> anyhow::Result<bool> {
        sqlx::query("SELECT 1")
            .execute(self.0.pool().as_ref())
            .await?;
        Ok(true)
    }
}
