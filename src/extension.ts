import * as vscode from "vscode";
import { window } from "vscode";
import { ClientContext } from "./client";
import { ConnectionConfigManager } from "./config";
import { ConnectionController } from "./controller/connectionController";
import { WorkspaceController } from "./controller/workspaceController";
import { StatusBarView } from "./views/statusBar";

const clientTraceName = "DB Viewer Client";

let client: ClientContext | undefined = undefined;
let connectionManager: ConnectionConfigManager | undefined = undefined;
let statusBar: StatusBarView | undefined = undefined;
let connectionController: ConnectionController | undefined = undefined;
let workspaceController: WorkspaceController | undefined = undefined;

export function activate(context: vscode.ExtensionContext) {
  const traceOutputChannel = window.createOutputChannel(clientTraceName);
  client = new ClientContext(context, traceOutputChannel);
  connectionManager = new ConnectionConfigManager(context);
  connectionController = new ConnectionController(
    context,
    client,
    connectionManager
  );
  workspaceController = new WorkspaceController(
    context,
    client,
    connectionManager
  );
  statusBar = new StatusBarView(context, connectionManager);

  connectionManager.getSelectedConnection();

  // start the server
  client.startServer();
}

export function deactivate() {
  if (client) {
    client.stopServer();
  }
}
