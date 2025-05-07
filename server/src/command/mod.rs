use cmd::{CheckConnectionCommand, ExecuteCommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_lsp::lsp_types::ExecuteCommandParams;

pub mod cmd;

pub fn commands() -> Vec<Box<dyn Command + Send + Sync>> {
    vec![Box::new(ExecuteCommand), Box::new(CheckConnectionCommand)]
}

#[tower_lsp::async_trait]
pub trait Command {
    fn command(&self) -> &'static str;

    async fn handler(&self, params: ExecuteCommandParams) -> anyhow::Result<Option<CommandResult>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    data: Value,
    // 执行时间（毫秒）
    execution_time: f64,
}

impl CommandResult {
    pub fn try_create<T: Serialize>(data: T, execution_time: f64) -> anyhow::Result<Self> {
        Ok(CommandResult {
            data: serde_json::to_value(data)?,
            execution_time,
        })
    }
}
