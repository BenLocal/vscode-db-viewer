use std::time::Duration;

use base64::Engine;
use sqlx::{Column, MySql, Row, TypeInfo, mysql::MySqlPoolOptions};

use super::{
    ConnectionPool,
    connection::{DBConnectionOptions, DBSet, DatabaseManager, DatabaseOperations},
};

#[tower_lsp::async_trait]
impl DatabaseManager<MySql> for DBSet<MySql> {
    async fn create(options: &DBConnectionOptions) -> anyhow::Result<DBSet<MySql>> {
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect_lazy(&options.connection_string)?;

        Ok(DBSet::new(pool))
    }
}

impl Into<ConnectionPool> for DBSet<MySql> {
    fn into(self) -> ConnectionPool {
        Box::new(MySQLOperations(self))
    }
}

/// MySQL specific operations
pub struct MySQLOperations(DBSet<MySql>);

#[tower_lsp::async_trait]
impl DatabaseOperations for MySQLOperations {
    async fn execute_query(&self, query: &str) -> anyhow::Result<(serde_json::Value, usize)> {
        // For SELECT queries, fetch rows
        if query.trim().to_lowercase().starts_with("select") {
            let rows = sqlx::query(query).fetch_all(self.0.pool().as_ref()).await?;
            let total = rows.len();
            let mut result = Vec::new();
            for row in rows {
                let mut obj = serde_json::Map::new();

                // Convert each column to a JSON value
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name();
                    // 这里直接尝试获取值作为字符串表示
                    let value = if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        match val {
                            Some(s) => serde_json::Value::String(s),
                            None => serde_json::Value::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(i) {
                        // 对于二进制数据特殊处理
                        match val {
                            Some(bytes) => {
                                let base64_str =
                                    base64::engine::general_purpose::STANDARD.encode(&bytes);
                                serde_json::Value::String(format!("(binary) {}", base64_str))
                            }
                            None => serde_json::Value::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                        // 对于整数类型
                        match val {
                            Some(n) => serde_json::Value::String(n.to_string()),
                            None => serde_json::Value::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                        // 对于浮点类型
                        match val {
                            Some(n) => serde_json::Value::String(n.to_string()),
                            None => serde_json::Value::Null,
                        }
                    } else {
                        // 如果所有尝试都失败，返回类型信息
                        let type_info = column.type_info();
                        serde_json::Value::String(format!("(unknown type: {})", type_info.name()))
                    };

                    obj.insert(column_name.to_string(), value);
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
        let rows = sqlx::query("SHOW TABLES")
            .fetch_all(self.0.pool().as_ref())
            .await?;

        let mut tables = Vec::new();
        for row in rows {
            // Handle VARBINARY column type by first getting it as bytes
            let table_name_bytes: Vec<u8> = row.try_get(0)?;
            // Then convert bytes to string, replacing invalid UTF-8 sequences
            let table_name = String::from_utf8_lossy(&table_name_bytes).to_string();
            tables.push(table_name);
        }

        Ok(tables)
    }

    async fn get_columns(&self, table_name: &str) -> anyhow::Result<Vec<String>> {
        let query = format!("SHOW COLUMNS FROM {}", table_name);
        let rows = sqlx::query(&query)
            .fetch_all(self.0.pool().as_ref())
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            // Also handle Field column the same way
            let column_name_bytes: Vec<u8> = row.try_get("Field")?;
            let column_name = String::from_utf8_lossy(&column_name_bytes).to_string();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::DBConnectionOptions;

    #[tokio::test]
    async fn test_mysql_operations() {
        let options = DBConnectionOptions {
            connection_string: "mysql://root:root@localhost:3306/test".to_string(),
            ..Default::default()
        };

        let table = "user";
        let db_set = DBSet::<MySql>::create(&options).await.unwrap();
        let operations = MySQLOperations(db_set);

        // Test execute_query
        let result = operations
            .execute_query(&format!("SELECT * FROM {}", table))
            .await
            .unwrap();
        println!("{:?}", result);

        // Test get_tables
        let tables = operations.get_tables().await.unwrap();
        println!("{:?}", tables);

        // Test get_columns
        let columns = operations.get_columns(table).await.unwrap();
        println!("{:?}", columns);

        // Test check_connection
        let is_connected = operations.check_connection().await.unwrap();
        assert!(is_connected);
    }
}
