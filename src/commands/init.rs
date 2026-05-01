use std::path::Path;

use crate::error::CliResult;
use crate::output;
use crate::store::Store;

pub fn run(root: &Path, rebuild: bool) -> CliResult<()> {
    let (init_patch, rebuilt) = if rebuild {
        let store = Store::rebuild(root)?;
        let patch = store.root().join(".memory/sql/000001_init.sql");
        (patch, true)
    } else {
        let store = Store::open(root)?;
        let patch = store.root().join(".memory/sql/000001_init.sql");
        (patch, false)
    };

    output::print_init(&mut std::io::stdout(), root, rebuilt, &init_patch);
    Ok(())
}
