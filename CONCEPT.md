# Data Model

* Primary Directory in a codebase: `.memory/`

This contains an SQLite database and documents in `.memory/documents`
The database holds metadata and points to these (plaintext) documents. 

Metadata includes: 
* Document Path
* Creation Date
* Related Files
* Related Documents
* Invalidated (At some point, it was noticed that this is not accurate anymore)
* Invalidation Reason
* Quick Summary
* Type (i.e. `COMMIT`, `PLAN`, `RESEARCH`) -> Memories created from three different types of actions

This is also builds a relational graph between documents and files, therefore pointing transitively to other documents.
Every query yields all directly relevant documents and documents one level deeper. One level deeper only shows the summaries and the id to directly query it.

## CLI
This is primarily intended to be used by agents. So the CLI interface must be intuitive. 
So that means something like
* `memorybank query-files path/to/foo.ts path/to/bar.ts`
* `memorybank query-research "Research Topic"` 
* `memorybank query-plans "Term to grep for"`
* `memorybank read <document id>` -> Also transitively suggests related documents (summaries and IDs)
* `memorybank add` takes a JSON from STDIN with document text, summary, related files, related docuemnts and type.

Outputs should be easily legible and well formated markdown. 