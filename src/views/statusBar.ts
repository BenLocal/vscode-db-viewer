import * as vscode from "vscode";
import { ClientContext } from "../client";
import { ConnectionConfigManager } from "../config";

export class StatusBarView {
  private context: vscode.ExtensionContext;
  private connectItem: vscode.StatusBarItem;

  constructor(
    context: vscode.ExtensionContext,
    private connectionManager: ConnectionConfigManager
  ) {
    this.context = context;
    // Create status bar item
    this.connectItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left,
      100
    );

    this.connectItem.command = "dbviewer.listConnections";
    context.subscriptions.push(this.connectItem);

    // Update status bar with initial state
    this.updateStatusBar();

    // Listen for connection selection changes
    connectionManager.onDidChangeSelectedConnection(() => {
      this.updateStatusBar();
    });

    // Show the status bar item
    this.connectItem.show();
    this.context.subscriptions.push(this.connectItem);
  }

  private updateStatusBar(): void {
    const connection = this.connectionManager.getSelectedConnection();

    if (connection) {
      this.connectItem.text = `$(database) ${connection.name}`;
      this.connectItem.tooltip = `Connected to: ${connection.connectionString}`;
    } else {
      this.connectItem.text = "$(database) No DB Connected";
      this.connectItem.tooltip = "Click to select a database connection";
    }
  }

  // For cleanup when extension deactivates
  public dispose(): void {
    this.connectItem.dispose();
  }
}
