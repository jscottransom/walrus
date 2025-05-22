use std::fs::File;
use std::io::Result;
use tempfile::tempdir;

use walrus::log::store;

fn test_new_store() -> Result<()> {
    let dir = tempdir()?;
    let file_path = dir.path().join("test-new-store.txt");
    let file = File::create(file_path)?;

    let store = store::new(&file);

    drop(file);
    dir.close()?;
    Ok(())
}
