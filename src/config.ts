import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

export interface DBConnection {
  name: string;
  connectionString: string;
  type?: string;
  username?: string;
  password?: string;
}

export class ConnectionConfigManager {
  private configPath: string;
  private connections: DBConnection[] = [];
  private fileWatcher: vscode.FileSystemWatcher | undefined;
  private onDidChangeEmitter = new vscode.EventEmitter<DBConnection[]>();
  private onDidChangeSelectedEmitter = new vscode.EventEmitter<
    DBConnection | undefined
  >();
  private extensionContext: vscode.ExtensionContext;

  // Public events that clients can subscribe to
  public readonly onDidChangeConnections = this.onDidChangeEmitter.event;
  public readonly onDidChangeSelectedConnection =
    this.onDidChangeSelectedEmitter.event;

  // Constants for storage keys
  private readonly SELECTED_CONNECTION_KEY = "selectedConnectionName";

  constructor(context: vscode.ExtensionContext) {
    this.extensionContext = context;
    // Store connections in the extension's global storage path
    this.configPath = path.join(
      context.globalStorageUri.fsPath,
      "db-connections.json"
    );

    // Ensure directory exists
    const dir = path.dirname(this.configPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    // Load connections from file
    this.loadConnections();

    // Set up file watcher
    this.setupFileWatcher(context);
  }

  /**
   * Sets up a file watcher to monitor changes to the config file
   */
  private setupFileWatcher(context: vscode.ExtensionContext): void {
    // Create file system watcher for the exact config file
    this.fileWatcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(
        path.dirname(this.configPath),
        path.basename(this.configPath)
      )
    );

    // When file changes, reload connections
    this.fileWatcher.onDidChange(() => {
      this.reloadConnections();
    });

    // When file is created (if deleted and recreated), reload connections
    this.fileWatcher.onDidCreate(() => {
      this.reloadConnections();
    });

    // Add to subscriptions to ensure disposal when extension is deactivated
    context.subscriptions.push(this.fileWatcher);
    context.subscriptions.push(this.onDidChangeEmitter);
    context.subscriptions.push(this.onDidChangeSelectedEmitter);
  }

  /**
   * Get the currently selected connection
   */
  public getSelectedConnection(): DBConnection | undefined {
    const selectedName = this.extensionContext.globalState.get<string>(
      this.SELECTED_CONNECTION_KEY
    );
    if (selectedName) {
      return this.getConnectionByName(selectedName);
    }
    return undefined;
  }

  /**
   * Set the currently selected connection
   */
  public setSelectedConnection(
    connection: DBConnection | string | undefined
  ): void {
    let connectionName: string | undefined;

    if (typeof connection === "string") {
      // If a string is provided, use it as the connection name
      connectionName = connection;
    } else if (connection) {
      // If a connection object is provided, use its name
      connectionName = connection.name;
    } else {
      // If undefined is provided, clear the selection
      connectionName = undefined;
    }

    // Store the selected connection name in global state
    this.extensionContext.globalState.update(
      this.SELECTED_CONNECTION_KEY,
      connectionName
    );

    // Emit change event with the full connection object
    const selectedConnection = connectionName
      ? this.getConnectionByName(connectionName)
      : undefined;
    this.onDidChangeSelectedEmitter.fire(selectedConnection);
  }

  /**
   * Reload connections from file system
   * This is called when the config file changes outside our code
   */
  public reloadConnections(): void {
    const previousConnections = [...this.connections]; // Save copy of previous state
    const selectedConnectionName =
      this.extensionContext.globalState.get<string>(
        this.SELECTED_CONNECTION_KEY
      );

    try {
      this.loadConnections();

      // Only emit change event if connections actually changed
      if (
        JSON.stringify(previousConnections) !== JSON.stringify(this.connections)
      ) {
        this.onDidChangeEmitter.fire(this.connections);

        // Check if the previously selected connection still exists
        if (selectedConnectionName) {
          const stillExists = this.connections.some(
            (conn) => conn.name === selectedConnectionName
          );
          if (!stillExists) {
            // Selected connection was removed, update selected state
            this.setSelectedConnection(undefined);
          }
        }
      }
    } catch (error) {
      vscode.window.showErrorMessage(
        `Failed to reload database connections: ${error}`
      );
    }
  }

  /**
   * Get the path to the configuration file
   */
  public getConfigPath(): string {
    return this.configPath;
  }

  /**
   * Get all stored database connections
   */
  public getConnections(): DBConnection[] {
    return this.connections;
  }

  /**
   * Get a specific database connection by name
   */
  public getConnectionByName(name: string): DBConnection | undefined {
    return this.connections.find((conn) => conn.name === name);
  }

  /**
   * Add or update a database connection
   */
  public saveConnection(connection: DBConnection): void {
    const existingIndex = this.connections.findIndex(
      (conn) => conn.name === connection.name
    );

    if (existingIndex >= 0) {
      // Update existing connection
      this.connections[existingIndex] = connection;
    } else {
      // Add new connection
      this.connections.push(connection);
    }

    this.saveConnections();
  }

  /**
   * Delete a database connection by name
   */
  public deleteConnection(name: string): boolean {
    const initialLength = this.connections.length;
    this.connections = this.connections.filter((conn) => conn.name !== name);

    if (this.connections.length !== initialLength) {
      this.saveConnections();

      // Emit change event
      this.onDidChangeEmitter.fire([...this.connections]);

      // Check if we deleted the selected connection
      const selectedName = this.extensionContext.globalState.get<string>(
        this.SELECTED_CONNECTION_KEY
      );
      if (selectedName === name) {
        this.setSelectedConnection(undefined);
      }

      return true;
    }

    return false;
  }

  /**
   * Load connections from the JSON file
   */
  private loadConnections(): void {
    try {
      if (fs.existsSync(this.configPath)) {
        const fileContent = fs.readFileSync(this.configPath, "utf8");
        const data = JSON.parse(fileContent);
        this.connections = Array.isArray(data) ? data : [];
      }
    } catch (error) {
      console.error("Failed to load database connections:", error);
      // Initialize with empty array on error
      this.connections = [];
    }
  }

  /**
   * Save connections to the JSON file
   */
  private saveConnections(): void {
    try {
      fs.writeFileSync(
        this.configPath,
        JSON.stringify(this.connections, null, 2),
        "utf8"
      );
    } catch (error) {
      vscode.window.showErrorMessage(
        `Failed to save database connections: ${error}`
      );
    }
  }
}
