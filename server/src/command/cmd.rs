use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_lsp::lsp_types::{ExecuteCommandParams, MessageType};

use crate::{
    constant::{SERVER_CHECK_CONNECTION, SERVER_EXECUTE_COMMAND},
    db::connection::DBConnectionOptions,
    logger::log,
};

use super::{Command, CommandResult};

// 定义SQL查询请求参数结构
#[derive(Debug, Deserialize)]
struct ExecuteQueryParams {
    query: String,
    #[serde(default)]
    connection_id: String,
    #[serde(default)]
    connection_string: String,
}

// 定义SQL查询结果结构
#[derive(Debug, Serialize)]
struct QueryResult {
    columns: Vec<String>,
    rows: serde_json::Value,
    affected_rows: usize,
}

#[derive(Debug)]
pub struct ExecuteCommand;

impl ExecuteCommand {
    // 执行SQL查询的实现
    async fn execute_sql_query(
        &self,
        query: &str,
        connection_id: &str,
        options: DBConnectionOptions,
    ) -> anyhow::Result<QueryResult> {
        let connect = crate::db::from_cache(connection_id, options).await;
        let pool = connect
            .get_pool()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get pool from connection"))?;
        let (res, total) = pool.execute_query(query).await?;

        Ok(QueryResult {
            columns: Vec::new(),
            rows: res,
            affected_rows: total,
        })
    }
}

#[tower_lsp::async_trait]
impl Command for ExecuteCommand {
    fn command(&self) -> &'static str {
        SERVER_EXECUTE_COMMAND
    }

    async fn handler(&self, params: ExecuteCommandParams) -> anyhow::Result<Option<CommandResult>> {
        let query_params =
            serde_json::from_value::<ExecuteQueryParams>(params.arguments[0].clone())?;

        log(
            MessageType::INFO,
            format!("Executing SQL query: {}", query_params.query),
        );

        // 记录开始时间
        let start_time = std::time::Instant::now();

        // 执行SQL查询
        let result = self
            .execute_sql_query(
                &query_params.query,
                &query_params.connection_id,
                DBConnectionOptions {
                    connection_string: query_params.connection_string,
                },
            )
            .await?;
        let execution_time = start_time.elapsed().as_secs_f64() * 1000.0;

        Ok(Some(CommandResult::try_create(result, execution_time)?))
    }
}

pub struct CheckConnectionCommand;

#[derive(Debug, Deserialize)]
struct CheckConnectionParams {
    #[serde(default)]
    connection_id: String,
    #[serde(default)]
    connection_string: String,
}

#[tower_lsp::async_trait]
impl Command for CheckConnectionCommand {
    fn command(&self) -> &'static str {
        SERVER_CHECK_CONNECTION
    }

    async fn handler(&self, params: ExecuteCommandParams) -> anyhow::Result<Option<CommandResult>> {
        let req = serde_json::from_value::<CheckConnectionParams>(params.arguments[0].clone())?;
        let connect = crate::db::from_cache(
            &req.connection_id,
            DBConnectionOptions {
                connection_string: req.connection_string,
            },
        )
        .await;
        let _pool = connect.get_pool().await.unwrap();
        let result = _pool.check_connection().await?;
        Ok(Some(CommandResult::try_create(
            json!({
                "result": result,
            }),
            0.0,
        )?))
    }
}
