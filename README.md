# vscode-db-viewer

## Description

The vscode-db-viewer is a Visual Studio Code extension that provides a user-friendly interface for managing and executing SQL statements against various database connections.

## Features

- Execute SQL statements directly from the editor.
- Manage multiple database connections.
- Open and edit configuration files for database connections.

## Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/vscode-db-viewer.git
   ```
2. Navigate to the project directory:
   ```
   cd vscode-db-viewer
   ```
3. Install the dependencies:
   ```
   npm install
   ```

## Usage

- Open a SQL file or create a new one.
- Use the command palette (Ctrl+Shift+P) to access the commands:
  - **DB Viewer: List Connections** - View all available database connections.
  - **DB Viewer: Add Connection** - Add a new database connection.
  - **DB Viewer: Delete Connection** - Remove an existing database connection.
  - **DB Viewer: Open Configuration File** - Open the configuration file for editing.
  - **Execute SQL Statement** - Execute the SQL code in the current editor.

## Development

To compile the extension, run:

```
npm run compile
```

To watch for changes and automatically compile:

```
npm run watch
```

To run tests:

```
npm test
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License

This project is licensed under the MIT License. See the LICENSE file for details.
