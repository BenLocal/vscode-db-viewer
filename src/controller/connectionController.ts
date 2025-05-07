import * as vscode from "vscode";
import { ClientContext } from "../client";
import { ConnectionConfigManager } from "../config";

export class ConnectionController {
  private client: ClientContext;
  private context: vscode.ExtensionContext;
  private connectionManager: ConnectionConfigManager;

  constructor(
    context: vscode.ExtensionContext,
    client: ClientContext,
    connectionManager: ConnectionConfigManager
  ) {
    this.context = context;
    this.client = client;
    this.connectionManager = connectionManager;
    // Register commands for database connections
    const listConnectionsCommand = vscode.commands.registerCommand(
      "dbviewer.listConnections",
      () => this.listDatabaseConnections()
    );

    const addConnectionCommand = vscode.commands.registerCommand(
      "dbviewer.addConnection",
      () => this.addDatabaseConnection()
    );

    const deleteConnectionCommand = vscode.commands.registerCommand(
      "dbviewer.deleteConnection",
      () => this.deleteDatabaseConnection()
    );

    const openConfigFileCommand = vscode.commands.registerCommand(
      "dbviewer.openConfigFile",
      () => this.openConfigFile()
    );

    this.context.subscriptions.push(
      listConnectionsCommand,
      addConnectionCommand,
      deleteConnectionCommand,
      openConfigFileCommand
    );
  }

  private async openConfigFile() {
    // Get the configuration file path from the connection manager
    const configPath = this.connectionManager.getConfigPath();

    if (!configPath) {
      vscode.window.showErrorMessage("Configuration file path not available.");
      return;
    }

    // Check if the file exists
    try {
      const fs = require("fs");
      if (!fs.existsSync(configPath)) {
        // If file doesn't exist, create an empty connections array
        fs.writeFileSync(configPath, JSON.stringify([], null, 2), "utf8");
      }

      // Open the file in VS Code
      const document = await vscode.workspace.openTextDocument(configPath);
      await vscode.window.showTextDocument(document);
    } catch (error) {
      vscode.window.showErrorMessage(
        `Failed to open configuration file: ${error}`
      );
    }
  }

  /**
   * Shows a list of configured database connections
   */
  private async listDatabaseConnections() {
    const connections = this.connectionManager.getConnections();

    if (connections.length === 0) {
      vscode.window.showInformationMessage(
        "No database connections configured. Use 'DB Viewer: Add Connection' command to add one."
      );
      return;
    }

    // Get currently selected connection for pre-selection in the QuickPick
    const selectedConnection = this.connectionManager.getSelectedConnection();

    const connectionItems = connections.map((conn) => ({
      label: conn.name,
      description: conn.connectionString,
      connection: conn,
      // Add a checkmark to the currently selected connection
      picked: selectedConnection
        ? conn.name === selectedConnection.name
        : false,
    }));

    const selectedItem = await vscode.window.showQuickPick(connectionItems, {
      placeHolder: "Select a database connection",
    });

    if (selectedItem) {
      // Set as selected connection
      this.connectionManager.setSelectedConnection(selectedItem.connection);

      // Send selected connection to server
      this.client.sendConnectionToServer(selectedItem.connection);

      vscode.window.showInformationMessage(
        `Connected to ${selectedItem.label}`
      );
    }
  }

  /**
   * Add a new database connection
   */
  private async addDatabaseConnection() {
    const name = await vscode.window.showInputBox({
      prompt: "Enter a name for this connection",
      placeHolder: "My Database",
    });

    if (!name) {
      return;
    }

    const dbType = await vscode.window.showQuickPick(
      ["mysql", "postgresql", "sqlite"],
      { placeHolder: "Select database type" }
    );

    if (!dbType) {
      return;
    }

    const connectionString = await vscode.window.showInputBox({
      prompt: "Enter connection string",
      placeHolder:
        dbType === "sqlite"
          ? "sqlite:path/to/database.db"
          : `${dbType}://username:password@hostname:port/database`,
    });

    if (!connectionString) {
      return;
    }

    // check if the connection string is valid
    // const isValid = this.connectionManager.validateConnectionString(
    //   connectionString,
    //   dbType
    // );
    // if (!isValid) {
    //   vscode.window.showErrorMessage(
    //     `Invalid connection string for ${dbType} database.`
    //   );
    //   return;
    // }

    // Save the new connection
    this.connectionManager.saveConnection({
      name,
      type: dbType,
      connectionString,
    });

    vscode.window.showInformationMessage(
      `Database connection "${name}" added!`
    );
  }

  /**
   * Delete a database connection
   */
  private async deleteDatabaseConnection() {
    const connections = this.connectionManager.getConnections();

    if (connections.length === 0) {
      vscode.window.showInformationMessage(
        "No database connections to delete."
      );
      return;
    }

    const connectionItems = connections.map((conn) => ({
      label: conn.name,
      description: conn.connectionString,
    }));

    const selectedItem = await vscode.window.showQuickPick(connectionItems, {
      placeHolder: "Select a connection to delete",
    });

    if (!selectedItem) {
      return;
    }

    const confirmed = await vscode.window.showWarningMessage(
      `Are you sure you want to delete the connection "${selectedItem.label}"?`,
      "Yes",
      "No"
    );

    if (confirmed === "Yes") {
      const deleted = this.connectionManager.deleteConnection(
        selectedItem.label
      );
      if (deleted) {
        vscode.window.showInformationMessage(
          `Connection "${selectedItem.label}" deleted.`
        );
      }
    }
  }

  /**
   * Send all configured connections to the server on startup
   */
  private passConnectionSettingsToServer() {
    const connections = this.connectionManager.getConnections();
    if (connections.length > 0) {
      this.client.sendAllConnectionsToServer(connections);
    }
  }
}
