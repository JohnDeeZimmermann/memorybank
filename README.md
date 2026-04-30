# memorybank

A CLI tool for storing and querying semantic memory about a codebase.

It holds plaintext memory documents under `.memory/documents/` with SQLite metadata and replayable SQL patch files. Designed for use by AI agents.

## Quick Start

```sh
# Initialize memory bank in the current directory
memorybank init

# Add a memory document via JSON on stdin
memorybank add << 'EOF'
{
  "document": "# My Note\n\nSome content here.",
  "summary": "A short summary",
  "related_files": ["src/main.rs"],
  "related_documents": [],
  "type": "COMMIT"
}
EOF

# Read a document by its ID
memorybank read <id>

# Find documents associated with specific files
memorybank query-files src/main.rs Cargo.toml

# Search research memories
memorybank query-research "sqlite migrations"

# Search plan memories
memorybank query-plans "cli output"

# Rebuild SQLite cache from .sql patches
memorybank init --rebuild
```

All output is Markdown. Errors use a stable `ERROR: CODE message` format.

## Document Types

- `COMMIT` — commit messages and code changes
- `PLAN` — implementation plans and design docs
- `RESEARCH` — research findings and notes

## JSON Input for `add`

| Field | Required | Description |
|-------|----------|-------------|
| `document` | yes | Document body (Markdown) |
| `summary` | yes | Short summary |
| `type` | yes | One of `COMMIT`, `PLAN`, `RESEARCH` |
| `related_files` | no | File paths relative to root |
| `related_documents` | no | UUIDs of existing documents |
