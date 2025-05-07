import path from "path";
import * as vscode from "vscode";
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import { DBConnection } from "./config";

const SERVER_NAME = "DB Viewer Server";

export class ClientContext {
  private context: vscode.ExtensionContext;
  private outputChannel: vscode.OutputChannel;
  private run: Executable;
  private serverOptions: ServerOptions;
  private clientOptions: LanguageClientOptions;
  private client: LanguageClient;
  private state: boolean = false;

  constructor(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel
  ) {
    this.context = context;
    this.outputChannel = outputChannel;
    const ext = process.platform === "win32" ? ".exe" : "";
    const command = path.join(
      process.env.SERVER_BASE_PATH || __dirname,
      `db-viewer-server${ext}`
    );
    this.run = {
      command,
      options: {
        env: {
          ...process.env,
          RUST_LOG: "debug",
        },
      },
      transport: TransportKind.stdio,
    };
    this.serverOptions = {
      run: this.run,
      debug: this.run,
    };

    this.clientOptions = {
      documentSelector: [{ scheme: "file", language: "sql" }],
      synchronize: {
        fileEvents: vscode.workspace.createFileSystemWatcher("**/*.sql"),
      },
      traceOutputChannel: this.outputChannel,
    };
    this.client = new LanguageClient(
      SERVER_NAME,
      SERVER_NAME,
      this.serverOptions,
      this.clientOptions
    );
  }

  async startServer() {
    if (this.state) {
      return;
    }

    try {
      await this.client.start();
    } catch (error) {
      this.state = false;
      return;
    }

    this.state = true;
  }

  async stopServer() {
    if (!this.state || !this.client) {
      return;
    }

    try {
      await this.client.stop();
    } catch (error) {
      return;
    }
    this.state = false;
  }

  async restartServer() {
    if (!this.client) {
      return;
    }

    try {
      await this.client.restart();
    } catch (error) {
      return;
    }
    this.state = true;
  }

  async sendExecuteCommand(command: string, args: any[]) {
    return await this.client.sendRequest("workspace/executeCommand", {
      command: command,
      arguments: args,
    });
  }

  /**
   * Send a single database connection to the server
   */
  public sendConnectionToServer(connection: DBConnection): void {
    const params = {
      connectionId: connection.name,
      connectionString: connection.connectionString,
      type: connection.type,
      username: connection.username,
      password: connection.password,
    };

    this.client.sendRequest("db.registerConnection", params);
  }

  /**
   * Send all database connections to the server
   */
  public sendAllConnectionsToServer(connections: DBConnection[]): void {
    const params = {
      connections: connections.map((conn) => ({
        connectionId: conn.name,
        connectionString: conn.connectionString,
        type: conn.type,
        username: conn.username,
        password: conn.password,
      })),
    };

    this.client.sendRequest("db.registerAllConnections", params);
  }
}
