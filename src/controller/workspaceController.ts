import * as vscode from "vscode";
import { ClientContext } from "../client";
import { Constant } from "../constant";
import { ConnectionConfigManager } from "../config";

export class WorkspaceController {
  private context: vscode.ExtensionContext;
  private client: ClientContext;
  private connectionManager: ConnectionConfigManager;
  private sqlOutputChannel: vscode.OutputChannel;

  constructor(
    context: vscode.ExtensionContext,
    client: ClientContext,
    connectionManager: ConnectionConfigManager
  ) {
    this.context = context;
    this.client = client;
    this.connectionManager = connectionManager;

    // Create a dedicated output channel for SQL results
    this.sqlOutputChannel = vscode.window.createOutputChannel("SQL Results");

    const sqlExecuteCommand = vscode.commands.registerCommand(
      Constant.EXECUTE_COMMAND,
      (sqlStatement) => this.sqlExecute(sqlStatement)
    );

    this.context.subscriptions.push(sqlExecuteCommand);
    this.context.subscriptions.push(this.sqlOutputChannel);
  }

  /**
   * Registers the SQL execute command.
   * @returns {vscode.Disposable} The disposable object for the command.
   */
  private async sqlExecute(sqlStatement: string) {
    try {
      const select = this.connectionManager.getSelectedConnection();
      if (!select) {
        vscode.window.showErrorMessage("No database connection selected.");
        return;
      }
      // Show executing status
      this.sqlOutputChannel.clear();
      this.sqlOutputChannel.appendLine(
        `Executing SQL on database: ${select.name}`
      );
      this.sqlOutputChannel.appendLine(`SQL: ${sqlStatement}`);
      this.sqlOutputChannel.appendLine("");
      this.sqlOutputChannel.show(true); // Preserve focus

      const res = await this.client.sendExecuteCommand(
        Constant.SERVER_EXECUTE_COMMAND,
        [
          {
            query: sqlStatement,
            connection_id: select.name,
            connection_string: select.connectionString,
          },
        ]
      );
      if (res) {
        // Display results in a formatted way
        this.displayResults(res, sqlStatement);
      } else {
        this.sqlOutputChannel.appendLine("⚠️ No results returned.");
      }
    } catch (error) {
      this.sqlOutputChannel.appendLine(
        `❌ Failed to execute SQL: ${sqlStatement}`
      );
      this.sqlOutputChannel.appendLine(
        `\nError details:\n${
          error instanceof Error ? error.message : String(error)
        }${
          error instanceof Error && "data" in error
            ? `\n${(error as any).data}`
            : ""
        }`
      );
      this.sqlOutputChannel.appendLine(
        `\nPlease check the SQL statement and try again.`
      );
      this.sqlOutputChannel.appendLine(`  Possible reasons:`);
      this.sqlOutputChannel.appendLine(`    - SQL syntax error`);
      this.sqlOutputChannel.appendLine(`    - SQL connection error`);
      this.sqlOutputChannel.appendLine(`    - SQL execution error`);
    }
  }

  /**
   * Display SQL results in appropriate format
   * @param res Result data
   * @param sqlStatement Original SQL statement
   */
  private displayResults(res: any, sqlStatement: string): void {
    // Clear previous results
    this.sqlOutputChannel.show(true);
    const data = res.data;
    const execution_time = res.execution_time || "";

    // Handle SELECT query results (returns an array)
    if (Array.isArray(data.rows)) {
      this.displaySelectResults(data.rows, execution_time);
    }
    // Handle DML operations (returns an object with rowsAffected)
    else if (typeof res === "object" && res !== null && "rowsAffected" in res) {
      this.sqlOutputChannel.appendLine(
        `✅ Success: ${res.rowsAffected} row(s) affected, execution time: ${execution_time} ms`
      );
    }
    // Handle other result types
    else {
      this.sqlOutputChannel.appendLine("Result:");
      this.sqlOutputChannel.appendLine(JSON.stringify(res, null, 2));
    }
  }

  /**
   * Display SELECT query results in tabular format
   * @param rows Array of result rows
   * @param execution_time Execution time of the query
   */
  private displaySelectResults(rows: any[], execution_time: string): void {
    if (rows.length === 0) {
      this.sqlOutputChannel.appendLine("Query returned 0 rows.");
      return;
    }

    // Extract column names from the first row
    const columns = Object.keys(rows[0]);

    if (columns.length === 0) {
      this.sqlOutputChannel.appendLine("Query returned rows with no columns.");
      return;
    }

    // Calculate column widths (min 10, max 50)
    const columnWidths: { [key: string]: number } = {};
    columns.forEach((col) => {
      // Start with column name length
      columnWidths[col] = Math.max(10, col.length);

      // Check each row's data width
      rows.forEach((row) => {
        const cellValue = this.formatCellValue(row[col]);
        columnWidths[col] = Math.min(
          50,
          Math.max(columnWidths[col], cellValue.length)
        );
      });
    });

    // Render table header
    let header = "| ";
    let separator = "| ";

    columns.forEach((col) => {
      header += col.padEnd(columnWidths[col]) + " | ";
      separator += "-".repeat(columnWidths[col]) + " | ";
    });

    this.sqlOutputChannel.appendLine(header);
    this.sqlOutputChannel.appendLine(separator);

    // Render table rows
    rows.forEach((row, index) => {
      let line = "| ";

      columns.forEach((col) => {
        const cellValue = this.formatCellValue(row[col]);
        line += cellValue.padEnd(columnWidths[col]) + " | ";
      });

      this.sqlOutputChannel.appendLine(line);

      // Add a separator every 20 rows for readability with large result sets
      if (
        rows.length > 30 &&
        index > 0 &&
        index % 20 === 0 &&
        index < rows.length - 1
      ) {
        this.sqlOutputChannel.appendLine(separator);
      }
    });

    // Show row count
    this.sqlOutputChannel.appendLine("");
    this.sqlOutputChannel.appendLine(
      `✅ ${rows.length} row(s) returned, execution time: ${execution_time} ms`
    );
  }

  /**
   * Format cell value for display
   * @param value Cell value
   * @returns Formatted string value
   */
  private formatCellValue(value: any): string {
    if (value === null || value === undefined) {
      return "NULL";
    } else if (typeof value === "object") {
      try {
        return JSON.stringify(value);
      } catch {
        return "[Object]";
      }
    } else if (typeof value === "string") {
      // Truncate long strings
      return value.length > 100 ? value.substring(0, 97) + "..." : value;
    } else {
      return String(value);
    }
  }
}
