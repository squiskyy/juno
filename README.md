# Juno

Windows-native AI agent with a chat interface, powered by local Ollama inference.

Juno adds tools on top of Ollama for file system work, MCP connectors, and computer
use - all through a sleek native Windows UI built with Tauri + Rust.

## Features

- **Local AI** - Uses Ollama models running on your own machine
- **Workspace Folders** - Add folders to the chat context so Juno can read and search files
- **Tool System** - Built-in tools: read_file, write_file, list_directory, search_files, shell_command
- **Computer Use** (Windows only) - Take screenshots, click, and type on the desktop
- **MCP Connector** - Experimental support for Model Context Protocol servers
- **Dark Theme** - Clean dark-mode chat UI

## Prerequisites

- [Ollama](https://ollama.com) installed and running locally
- At least one model pulled (`ollama pull llama3.2` or similar)
- Windows 10+ for computer-use features

## Development

```bash
cd src-tauri
cargo tauri dev
```

## Building

```bash
cd src-tauri
cargo tauri build
```

The installer will be in `src-tauri/target/release/bundle/`.

## License

MIT
