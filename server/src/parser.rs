use std::vec;

use sqlparser::{ast::Spanned, dialect::GenericDialect};
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};

use crate::constant::CLIENT_EXECUTE_COMMAND;

#[derive(Debug, Clone)]
/// Represents a SQL AST (Abstract Syntax Tree).
pub struct SqlAst(Vec<sqlparser::ast::Statement>);

pub enum CompletionContext {
    None,
    TableName,
    ColumnName(String), // 包含表名
}

impl SqlAst {
    pub fn code_lens(&self) -> anyhow::Result<Option<Vec<CodeLens>>> {
        let mut code_lens = vec![];
        for statement in &self.0 {
            match statement {
                sqlparser::ast::Statement::Query(_)
                | sqlparser::ast::Statement::Insert(_)
                | sqlparser::ast::Statement::Update { .. }
                | sqlparser::ast::Statement::Delete(_)
                | sqlparser::ast::Statement::CreateTable { .. } => {
                    let command = Command {
                        title: "😼 Run SQL".to_string(),
                        command: CLIENT_EXECUTE_COMMAND.to_string(),
                        // 将SQL语句作为参数传递给命令
                        arguments: Some(vec![serde_json::to_value(statement.to_string()).unwrap()]),
                    };
                    code_lens.push(CodeLens {
                        range: Range {
                            start: Position {
                                line: (statement.span().start.line - 1) as u32,
                                character: 0,
                            },
                            end: Position {
                                line: (statement.span().end.line - 1) as u32,
                                character: statement.span().end.column as u32,
                            },
                        },
                        command: Some(command),
                        data: None,
                    });
                }
                _ => {}
            }
        }

        Ok(Some(code_lens))
    }

    pub fn get_completion_context(&self, position: Position) -> CompletionContext {
        // 根据光标位置和SQL AST分析当前上下文
        // 这需要深入解析SQL语法，但可以简化为一些基本模式匹配

        // 例如：如果光标在FROM或JOIN后面，则为TableName上下文
        // 如果光标在表名后面跟着点(.)，则为ColumnName上下文

        // 实现细节依赖于您的SQL解析器

        // 示例简化实现：
        let line = position.line as usize;
        let character = position.character as usize;

        // 获取当前行的文本
        // if let Some(stmt) = self.get_statement_at(line, character) {
        //     let line_text = stmt.text.lines().nth(position.line as usize).unwrap_or("");
        //     let prefix = &line_text[0..character as usize];

        //     // 简单匹配：在FROM或JOIN后面提示表名
        //     if prefix.to_uppercase().trim().ends_with("FROM")
        //         || prefix.to_uppercase().contains("JOIN")
        //     {
        //         return CompletionContext::TableName;
        //     }

        //     // 简单匹配：在表名后面的点后提示列名
        //     if let Some(table_name) = Self::extract_table_name_before_dot(prefix) {
        //         return CompletionContext::ColumnName(table_name);
        //     }
        // }

        CompletionContext::None
    }

    // 辅助函数：提取点号前的表名
    fn extract_table_name_before_dot(text: &str) -> Option<String> {
        // 这是一个简化实现，实际应用中需要更复杂的解析
        let parts: Vec<&str> = text.trim().split('.').collect();
        if parts.len() >= 2 {
            let potential_table = parts[parts.len() - 2].trim();
            // 从后往前查找第一个空格或特殊字符
            if let Some(pos) =
                potential_table.rfind(|c: char| c.is_whitespace() || "(),;".contains(c))
            {
                return Some(potential_table[pos + 1..].trim().to_string());
            } else {
                return Some(potential_table.to_string());
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct SqlParser {
    dialect: GenericDialect,
}

impl SqlParser {
    pub(crate) fn new() -> Self {
        SqlParser {
            dialect: GenericDialect {},
        }
    }

    pub(crate) fn parse(&self, sql: &str) -> anyhow::Result<SqlAst> {
        let ast = sqlparser::parser::Parser::parse_sql(&self.dialect, sql)?;
        Ok(SqlAst(ast))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_parser() {
        let parser = SqlParser::new();
        let sql = "
        SELECT * FROM users WHERE id = 1

        AND name = 'John Doe';

        INSERT INTO users (name, age) VALUES ('Jane Doe', 30);
        UPDATE users SET age = 31 WHERE name = 'Jane Doe';
        DELETE FROM users WHERE name = 'John Doe';
        CREATE TABLE orders (id INT, user_id INT, amount DECIMAL);
        ";
        let result = parser.parse(sql).unwrap();
        let code_lens = result.code_lens().unwrap().unwrap();
        assert_eq!(code_lens.len(), 5);

        for code_len in code_lens {
            assert_eq!(code_len.command.as_ref().unwrap().title, "😼 Run SQL");
            assert_eq!(code_len.command.as_ref().unwrap().command, "sql.execute");
            assert!(code_len.command.as_ref().unwrap().arguments.is_some());
            let args = code_len
                .command
                .as_ref()
                .unwrap()
                .arguments
                .as_ref()
                .unwrap();
            assert_eq!(args.len(), 1);
            let sql = args[0]
                .as_str()
                .unwrap_or_else(|| panic!("Expected a string, got: {:?}", args[0]));
            assert!(
                sql.contains("SELECT")
                    || sql.contains("INSERT")
                    || sql.contains("UPDATE")
                    || sql.contains("DELETE")
                    || sql.contains("CREATE")
            );
        }
    }
}
