# Memory Bank

Memory bank is a CLI tool meant to be used by agents. 
It holds a semantic history over a codebase. 

In codebase's root directory, a .memory directory is held.
This contains an SQLite database and so called documents in `.memory/documents/`.

These documents are atomic entries in memory. The database points to the documents and holds metadata.