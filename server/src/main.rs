#![deny(clippy::disallowed_macros)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

use std::collections::HashMap;
use std::sync::Arc;

use command::Command;
use parser::{CompletionContext, SqlAst, SqlParser};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::{
    CodeLens, CodeLensOptions, CodeLensParams, CompletionOptions, CompletionParams,
    ExecuteCommandOptions, ExecuteCommandParams, InitializedParams, MessageType,
    ServerCapabilities, TextDocumentSyncKind,
};
use tower_lsp::{Client, LspService};
use tower_lsp::{
    LanguageServer, Server,
    lsp_types::{InitializeParams, InitializeResult},
};

mod command;
mod constant;
mod db;
mod logger;
mod parser;

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}

struct Backend {
    client: Arc<Client>,
    document_map: Arc<RwLock<HashMap<String, SqlAst>>>,
    sql_parser: SqlParser,
    commands: Vec<Box<dyn Command + Send + Sync>>,

    cancel: CancellationToken,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.log_message_spawn();
        let capabilities = ServerCapabilities {
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                resolve_provider: Some(false),
                ..Default::default()
            }),
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(false),
            }),
            text_document_sync: Some(tower_lsp::lsp_types::TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: self
                    .commands
                    .iter()
                    .map(|cmd| cmd.command().to_string())
                    .collect(),
                work_done_progress_options: Default::default(),
            }),
            ..ServerCapabilities::default()
        };
        Ok(InitializeResult {
            capabilities,
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "server shutdown!")
            .await;
        self.cancel();
        Ok(())
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let document_uri = params.text_document.uri.to_string();
        let document_map = self.document_map.read().await;

        if let Some(content) = document_map.get(&document_uri) {
            content.code_lens().map_err(|e| Error {
                code: ErrorCode::InternalError,
                message: "Failed to generate CodeLens".to_string().into(),
                data: Some(e.to_string().into()),
            })
        } else {
            Ok(None)
        }
    }

    // 实现文档同步，以便跟踪文档内容
    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let ast = match self.sql_parser.parse(&params.text_document.text) {
            Ok(ast) => ast,
            Err(_) => return,
        };

        {
            let mut document_map = self.document_map.write().await;
            document_map.insert(params.text_document.uri.to_string(), ast);
        }

        // 通知客户端刷新CodeLens
        self.client
            .log_message(
                MessageType::INFO,
                "SQL document opened, refreshing CodeLens",
            )
            .await;

        self.client.code_lens_refresh().await.unwrap();
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        let change = match params.content_changes.first() {
            Some(change) => change,
            None => return,
        };
        let ast = match self.sql_parser.parse(&change.text) {
            Ok(ast) => ast,
            Err(_) => return,
        };

        {
            let mut document_map = self.document_map.write().await;
            document_map.insert(params.text_document.uri.to_string(), ast);
            self.client
                .log_message(
                    MessageType::INFO,
                    "SQL document changed, refreshing CodeLens",
                )
                .await;

            // 通知客户端刷新CodeLens
            self.client.code_lens_refresh().await.unwrap();
        }
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        let mut document_map = self.document_map.write().await;
        self.client
            .log_message(
                MessageType::INFO,
                "SQL document removed, refreshing CodeLens",
            )
            .await;
        document_map.remove(&params.text_document.uri.to_string());
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.commands
            .iter()
            .find(|cmd| cmd.command() == params.command)
            .ok_or_else(|| Error {
                code: ErrorCode::MethodNotFound,
                message: "Command not found".to_string().into(),
                data: None,
            })?
            .handler(params)
            .await
            .map(|result| {
                result.map(|res| serde_json::to_value(res).unwrap_or_else(|_| Value::Null))
            })
            .map_err(|e| Error {
                code: ErrorCode::InternalError,
                message: "Command execution failed".to_string().into(),
                data: Some(e.to_string().into()),
            })
    }

    // async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
    //     let document_uri = params.text_document_position.text_document.uri.to_string();
    //     let position = params.text_document_position.position;

    //     // 获取当前文档
    //     let document_map = self.document_map.read().await;
    //     let doc = match document_map.get(&document_uri) {
    //         Some(doc) => doc,
    //         None => return Ok(None),
    //     };

    //     // 分析当前光标位置的上下文
    //     let context = doc.get_completion_context(position);

    //     match context {
    //         CompletionContext::TableName => {
    //             // 提供表名列表
    //             let mut items = Vec::new();
    //             let schema_cache = self.schema_cache.read().await;

    //             // 遍历所有已知数据库连接的模式信息
    //             for (conn_id, schema) in schema_cache.iter() {
    //                 for (table_name, table_info) in &schema.tables {
    //                     items.push(CompletionItem {
    //                         label: table_name.clone(),
    //                         kind: Some(CompletionItemKind::CLASS),
    //                         detail: Some(format!("Table ({conn_id})")),
    //                         documentation: Some(Documentation::MarkupContent(MarkupContent {
    //                             kind: MarkupKind::MARKDOWN,
    //                             value: format!(
    //                                 "### Table: {}\n\nColumns:\n{}",
    //                                 table_name,
    //                                 table_info
    //                                     .columns
    //                                     .iter()
    //                                     .map(|c| format!(
    //                                         "- **{}**: {} {}{}",
    //                                         c.name,
    //                                         c.data_type,
    //                                         if c.is_primary { " (PK)" } else { "" },
    //                                         if c.is_nullable { "" } else { " NOT NULL" }
    //                                     ))
    //                                     .collect::<Vec<_>>()
    //                                     .join("\n")
    //                             ),
    //                         })),
    //                         ..Default::default()
    //                     });
    //                 }
    //             }

    //             Ok(Some(CompletionResponse::Array(items)))
    //         }
    //         CompletionContext::ColumnName(table_name) => {
    //             // 提供指定表的列名列表
    //             let mut items = Vec::new();
    //             let schema_cache = self.schema_cache.read().await;

    //             for schema in schema_cache.values() {
    //                 if let Some(table) = schema.tables.get(&table_name) {
    //                     for column in &table.columns {
    //                         items.push(CompletionItem {
    //                             label: column.name.clone(),
    //                             kind: Some(CompletionItemKind::FIELD),
    //                             detail: Some(format!("{} ({})", column.data_type, table_name)),
    //                             documentation: Some(Documentation::String(format!(
    //                                 "Column: {} \nType: {}\nTable: {}",
    //                                 column.name, column.data_type, table_name
    //                             ))),
    //                             ..Default::default()
    //                         });
    //                     }
    //                 }
    //             }

    //             Ok(Some(CompletionResponse::Array(items)))
    //         }
    //         CompletionContext::None => {
    //             // 无特定上下文时的通用建议（关键字等）
    //             let keywords = vec![
    //                 "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER",
    //                 "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET", "INSERT", "UPDATE",
    //                 "DELETE", "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "AS",
    //             ];

    //             let items = keywords
    //                 .into_iter()
    //                 .map(|kw| CompletionItem {
    //                     label: kw.to_string(),
    //                     kind: Some(CompletionItemKind::KEYWORD),
    //                     ..Default::default()
    //                 })
    //                 .collect();

    //             Ok(Some(CompletionResponse::Array(items)))
    //         }
    //     }
    // }
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client),
            document_map: Arc::new(RwLock::new(HashMap::new())),
            sql_parser: SqlParser::new(),
            commands: command::commands(),
            cancel: CancellationToken::new(),
        }
    }

    fn cancel(&self) {
        self.cancel.cancel();
    }

    fn log_message_spawn(&self) {
        let cancel = self.cancel.clone();
        let mut rx = logger::subscribe();
        let client_clone = self.client.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    Ok((t, v)) = rx.recv() => {
                        client_clone.log_message(t, v).await;
                    }
                }
            }
        });
    }
}
