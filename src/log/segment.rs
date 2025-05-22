use std::fs::{File, OpenOptions};
use std::io::Result;
use std::path::Path;
use super::{store, index, config};

pub struct Segment {
    store: store::SafeStore,
    index: index::Index,
    base_offset: u64,
    next_offset: u64,
    config: config::Config

}

pub fn new(dir: &str, base_off: u64, conf: config::Config) -> Result<Segment> {
    let store_path = format!("{}/{}.store", dir, base_off);
    let index_path = format!("{}/{}.index", dir, base_off);
    
    
    let store_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(&store_path)?;

   let store = store::new(&store_file.try_clone()?)?;

   let index_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&index_path)?;

   let index = index::new(&index_file.try_clone()?, &conf)?;
   
   let next_offset = match index.read(-1){
        Ok((off, _)) => base_off + off as u64 + 1,
        Err(_) => base_off,
    };

    Ok( Segment { store: store, index: index, base_offset: base_off, next_offset: next_offset, config: conf })



}