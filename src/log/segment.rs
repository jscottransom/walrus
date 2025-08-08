use super::{config, index, store};
use prost::Message;
use std::fs::{OpenOptions, remove_file};

// Custom Result type to match log.rs
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Include the generated Record type
include!(concat!(env!("OUT_DIR"), "/log.rs"));

pub struct Segment {
    store: store::SafeStore,
    index: index::Index,
    base_offset: u64,
    next_offset: u64,
    config: config::Config,
}

pub fn new(dir: &str, _path: String, base_off: u64, conf: config::Config) -> Result<Segment> {
    let store_path = format!("{}/{}.store", dir, base_off);
    let index_path = format!("{}/{}.index", dir, base_off);

    let store_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true)
        .open(&store_path)?;

    let store = store::new(&store_file.try_clone()?, store_path)?;

    let index_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&index_path)?;

    let index = index::new(&index_file.try_clone()?, index_path, &conf)?;

    // Calculate next_offset by reading the last entry in the index
    // If index is empty, next_offset = base_offset
    // If index has entries, next_offset = base_offset + last_relative_offset + 1
    let next_offset = match index.read(-1) {
        Ok((relative_offset, _)) => {
            // The relative offset is 0-based, so we add 1 to get the next offset
            base_off + relative_offset as u64 + 1
        },
        Err(_) => {
            // Index is empty, start at base_offset
            base_off
        },
    };

    Ok(Segment {
        store,
        index,
        base_offset: base_off,
        next_offset,
        config: conf,
    })
}

impl Segment {
    // Getter methods for private fields
    pub fn base_offset(&self) -> u64 {
        self.base_offset
    }

    pub fn next_offset(&self) -> u64 {
        self.next_offset
    }

    pub fn append(&mut self, record: &mut Record) -> Result<u64> {
        // Set the record's offset to the current next_offset
        let current_offset = self.next_offset;
        record.offset = current_offset;
        
        // Convert the record to bytes
        let bytes = record.encode_to_vec();

        // Append to store and get the position
        let mut safe_store = self.store.lock().unwrap();
        let (_, position) = safe_store.append(&bytes)?;

        // Calculate relative offset for index
        let relative_offset = (self.next_offset - self.base_offset) as u32;

        // Write to index
        self.index.write(relative_offset, position)?;
        
        // Increment next_offset
        self.next_offset += 1;

        Ok(record.offset)
    }

    pub fn read(&mut self, offset: u64) -> Result<Record> {
        // Validate offset is within this segment's range
        if offset < self.base_offset || offset >= self.next_offset {
            return Err(format!("Offset {} not found in segment (base: {}, next: {})", 
                             offset, self.base_offset, self.next_offset).into());
        }

        // Calculate relative offset for index lookup
        let relative_offset = (offset - self.base_offset) as i64;
        
        // Read from index to get position
        let (_, position) = self.index.read(relative_offset)?;

        // Read from store
        let mut safe_store = self.store.lock().unwrap();
        let bytes = safe_store.read(position)?;

        // Decode the record
        let record = Record::decode(&*bytes)?;
        Ok(record)
    }

    pub fn is_maxed(&mut self) -> bool {
        let safe_store = self.store.lock().unwrap();
        safe_store.size >= self.config.segment.max_store_bytes
            || self.index.size >= self.config.segment.max_index_bytes
    }

    pub fn remove(&mut self) -> Result<()> {
        self.close()?;
        
        // Remove index file
        remove_file(&self.index.path)?;
        
        // Remove store file
        let safe_store = self.store.lock().unwrap();
        remove_file(&safe_store.path)?;

        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        // Close index
        self.index.close()?;

        // Close store
        let mut safe_store = self.store.lock().unwrap();
        safe_store.close()?;
        
        Ok(())
    }
}
