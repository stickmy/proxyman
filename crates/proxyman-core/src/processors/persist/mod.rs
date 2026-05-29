use std::{fs, path::Path};

pub mod processor_persist;
pub mod value_persist;

pub fn ensure_dir<P: AsRef<Path>>(dir: P) -> Result<(), std::io::Error> {
    match fs::metadata(dir.as_ref()) {
        Ok(meta) => {
            if !meta.is_dir() {
                fs::create_dir(dir.as_ref())
            } else {
                Ok(())
            }
        }
        Err(_) => fs::create_dir(dir),
    }
}
