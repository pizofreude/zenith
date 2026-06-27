# @zenitheditor/zenith-mcp

NPM launcher for the Zenith MCP server.

This package installs the matching prebuilt `zenith` binary from GitHub Releases and exposes a
`zenith-mcp` command for MCP clients.

## Usage

Run the MCP server:

```bash
npx -y @zenitheditor/zenith-mcp
```

Pass arguments through to the underlying `zenith` binary:

```bash
npx -y @zenitheditor/zenith-mcp --help
```

Install globally:

```bash
npm install -g @zenitheditor/zenith-mcp
zenith-mcp
```

## MCP Registry

MCP Registry name: `mcp-name: io.github.zenitheditor/zenith`

The main project is at https://github.com/zenitheditor/zenith.
