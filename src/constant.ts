export class Constant {
  private static instance: Constant;

  // Private constructor to prevent instantiation
  private constructor() {}

  public static getInstance(): Constant {
    if (!Constant.instance) {
      Constant.instance = new Constant();
    }
    return Constant.instance;
  }

  // Constants
  public static readonly EXECUTE_COMMAND: string = "dbviewer.execute";
  public static readonly SERVER_EXECUTE_COMMAND: string =
    "dbviewer.server.executeCommand";
  public static readonly SERVER_CHECK_CONNECTION: string =
    "dbviewer.server.checkConnection";
}
