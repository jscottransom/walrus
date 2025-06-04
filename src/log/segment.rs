use super::{config, index, store};
use prost::Message;
use std::fs::{OpenOptions, remove_file};
use std::io::Result;
use std::os;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/log.v1.rs"));

pub struct Segment {
    store: store::SafeStore,
    index: index::Index,
    base_offset: u64,
    next_offset: u64,
    config: config::Config,
}

pub fn new(dir: &str, path: String, base_off: u64, conf: config::Config) -> Result<Segment> {
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

    let next_offset = match index.read(-1) {
        Ok((off, _)) => base_off + off as u64 + 1,
        Err(_) => base_off,
    };

    Ok(Segment {
        store: store,
        index: index,
        base_offset: base_off,
        next_offset: next_offset,
        config: conf,
    })
}


// build a

impl Segment {
    pub fn append(&mut self, record: Record) -> Result<u64> {
        // Convert the record to a slice of raw bytes
        let bytes = record.encode_to_vec();

        // Append the raw bytes to the store
        let mut safe_store = self.store.lock().unwrap();
        let (_, position) = safe_store.append(&bytes)?;

        let rel_offset = (self.next_offset - self.base_offset) as u32;

        self.index.write(rel_offset, position)?;
        self.next_offset += 1;

        Ok(record.offset)
    }

    pub fn read(&mut self, offset: u64) -> Result<Record> {
        // Read from the index at the given offset

        let index_pos = (offset - self.base_offset) as i64;
        let (_out, position) = self.index.read(index_pos)?;

        // Store ops
        let mut safe_store = self.store.lock().unwrap(); // come back to deal with this, map error to unable to lock store
        let bytes = safe_store.read(position)?;

        // Bytes returned is a vector, but the decode function only accepts a reference to a byte array
        let record = Record::decode(&*bytes)?;
        Ok(record)
    }

    pub fn is_maxed(&mut self) -> bool {
        let safe_store = self.store.lock().unwrap();

        return safe_store.size >= self.config.segment.max_store_bytes
            || self.index.size >= self.config.segment.max_index_bytes;
    }

    pub fn remove(&mut self) -> Result<()> {
        self.close()?;
        remove_file(&self.index.path)?;
        let safe_store = self.store.lock().unwrap();
        remove_file(&safe_store.path)?;

        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {

        let _ = self.index.close();

        let mut safe_store = self.store.lock().unwrap();
        let _ = safe_store.close();
        
        Ok(())
    }
}
