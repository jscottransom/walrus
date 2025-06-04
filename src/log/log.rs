use byteorder::{BigEndian, ByteOrder};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::io;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::str::FromStr;
use super::{config, segment};


// Abstraction around the Log that will be the entry point for writing to the Log
// And committing changes
pub struct Log {
    pub dir: String,
    pub config: config::Config,
    pub active_segment: segment::Segment,
    pub segments: Vec<segment::Segment>,
}

pub type SafeLog = Arc<Mutex<Log>>;



fn setup_log(dir: String) -> Result<()> {

    // Read through the given directory
    // 
    let mut base_offsets: Vec<u64> = Vec::new();
    let dir_path = Path::new(&dir);
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_os_string();
            let stem_str = file_name.to_string_lossy();
            let ext = Path::new(&file_name_str).extension().and_then(|ext| ext.to_str()).unwrap();
            let stem = stem_str.trim_end_matches(&format!(".{}", ext));
            let offset = u64::from_str(stem)?;
            base_offsets.push(offset);
        }
    }

    // sort the offsets
    base_offsets.sort();
    
    for (i, &offset) in base_offsets.iter().enumerate() {
        if i % 2 == 0 {
            
        }
    }


    Ok(())

}


fn new_log(dir: String, config: config::Config) -> SafeLog {
    if config.segment.max_store_bytes == 0 {
        config.segment.max_store_bytes = 1024
    }

    if config.segment.max_index_bytes == 0 {
        config.segment.max_index_bytes = 1024
    }

    
    
    
    
    
    
    
    let log = Arc::new(Mutex::new(Store {
        dir: dir,
        config: config,
        size: size,
        buf: writer,
    }))


}