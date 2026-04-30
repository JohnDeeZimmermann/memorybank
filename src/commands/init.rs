use std::fs;
use std::path::Path;

use crate::db;
use crate::error::{CliError, CliResult};
use crate::output;
use crate::paths;
use crate::sql_log::SqlPatchLog;

pub fn run(root: &Path, rebuild: bool) -> CliResult<()> {
    let init_patch = ensure(root, rebuild)?;

    output::print_init(root, rebuild, &init_patch);
    Ok(())
}

pub fn ensure(root: &Path, rebuild: bool) -> CliResult<std::path::PathBuf> {
    paths::ensure_memory_dirs(root)?;
    let log = SqlPatchLog::new(root);
    let init_patch = log.ensure_init_patch()?;

    if rebuild {
        let db_path = paths::database_path(root);
        if db_path.exists() {
            fs::remove_file(&db_path).map_err(|err| {
                CliError::Storage(format!("Unable to remove existing database: {err}"))
            })?;
        }
        let conn = db::open(root)?;
        log.replay_all(&conn)?;
    } else {
        let conn = db::open(root)?;
        db::initialize_schema(&conn)?;
    }

    Ok(init_patch)
}
