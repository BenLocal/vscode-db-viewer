use std::time::Duration;

use sqlx::{Column, Postgres, Row, postgres::PgPoolOptions};

use super::{
    ConnectionPool,
    connection::{DBConnectionOptions, DBSet, DatabaseManager, DatabaseOperations},
};

#[tower_lsp::async_trait]
impl DatabaseManager<Postgres> for DBSet<Postgres> {
    async fn create(options: &DBConnectionOptions) -> anyhow::Result<DBSet<Postgres>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect_lazy(&options.connection_string)?;

        Ok(DBSet::new(pool))
    }
}

impl Into<ConnectionPool> for DBSet<Postgres> {
    fn into(self) -> ConnectionPool {
        Box::new(PostgreSQLOperations(self))
    }
}

/// PostgreSQL specific operations
pub struct PostgreSQLOperations(DBSet<Postgres>);

#[tower_lsp::async_trait]
impl DatabaseOperations for PostgreSQLOperations {
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
            "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema'"
        )
        .fetch_all(self.0.pool().as_ref())
        .await?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.try_get("tablename")?;
            tables.push(table_name);
        }

        Ok(tables)
    }

    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<String>> {
        let query = "SELECT column_name FROM information_schema.columns WHERE table_name = $1";
        let rows = sqlx::query(query)
            .bind(table_name)
            .fetch_all(self.0.pool().as_ref())
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let column_name: String = row.try_get("column_name")?;
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
