use crate::log::config;
use memmap2::MmapMut;
use std::fs::File;
use std::io::{Error, ErrorKind};

// Custom Result type to match other modules
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const OFF_WIDTH: u64 = 4;
const POS_WIDTH: u64 = 8;
const ENT_WIDTH: u64 = OFF_WIDTH + POS_WIDTH;

pub struct Index {
    pub file: File,
    pub path: String,
    mmap: MmapMut,
    pub size: u64,
}

pub fn new(file: &File, path: String, conf: &config::Config) -> Result<Index> {
    let file_size = file.metadata()?.len() as u64;
    let file_obj = file.try_clone()?;
    
    // Ensure the file is large enough for the memory map
    let max_size = conf.segment.max_index_bytes;
    if file_size < max_size {
        file_obj.set_len(max_size)?;
    }
    
    let mmap = unsafe { MmapMut::map_mut(&file_obj)? };

    // Calculate the actual data size by finding the last valid entry
    let mut actual_size = 0;
    if file_size > 0 {
        // Try to read entries to determine the actual size
        let mut offset = 0;
        while offset * ENT_WIDTH < file_size {
            let position = offset * ENT_WIDTH;
            if position + ENT_WIDTH > file_size {
                break;
            }
            
            // Check if this entry is valid by reading the offset
            let offset_bytes = &mmap[position as usize..(position + OFF_WIDTH) as usize];
            let offset_value = u32::from_be_bytes(
                offset_bytes.try_into()
                    .map_err(|_| Box::new(Error::new(ErrorKind::InvalidData, "Invalid offset bytes")))?,
            );
            
            // If offset is 0 and this is not the first entry, this might be an empty entry
            if offset_value == 0 && offset > 0 {
                // Check if the position is also 0
                let pos_bytes = &mmap[(position + OFF_WIDTH) as usize..(position + ENT_WIDTH) as usize];
                let pos_value = u64::from_be_bytes(
                    pos_bytes.try_into()
                        .map_err(|_| Box::new(Error::new(ErrorKind::InvalidData, "Invalid position bytes")))?,
                );
                if pos_value == 0 {
                    break;
                }
            }
            
            actual_size = (offset + 1) * ENT_WIDTH;
            offset += 1;
        }
    }

    let index = Index {
        file: file_obj,
        path,
        mmap,
        size: actual_size,
    };
    Ok(index)
}

impl Index {
    pub fn close(&mut self) -> Result<()> {
        // Sync all changes to disk
        self.file.sync_all()?;
        
        // Don't truncate the file - keep the full size for the memory map
        // The actual data size is stored in the file metadata
        
        Ok(())
    }

    pub fn read(&self, offset: i64) -> Result<(u32, u64)> {
        if self.size == 0 {
            return Err(Box::new(Error::new(ErrorKind::UnexpectedEof, "Index is empty")));
        }

        let index: u32 = if offset == -1 {
            // Read the last entry
            (self.size as u32 / ENT_WIDTH as u32).saturating_sub(1)
        } else {
            if offset < 0 {
                return Err(Box::new(Error::new(ErrorKind::InvalidInput, "Negative offset")));
            }
            offset as u32
        };

        let position = (index as u64) * ENT_WIDTH;
        if self.size < position + ENT_WIDTH {
            return Err(Box::new(Error::new(
                ErrorKind::UnexpectedEof,
                "Index entry out of bounds",
            )));
        }

        // Read offset (u32)
        let offset_bytes = &self.mmap[position as usize..(position + OFF_WIDTH) as usize];
        let out = u32::from_be_bytes(
            offset_bytes.try_into()
                .map_err(|_| Box::new(Error::new(ErrorKind::InvalidData, "Invalid offset bytes")))?,
        );

        // Read position (u64)
        let position_bytes = &self.mmap[(position + OFF_WIDTH) as usize..(position + ENT_WIDTH) as usize];
        let new_position = u64::from_be_bytes(
            position_bytes.try_into()
                .map_err(|_| Box::new(Error::new(ErrorKind::InvalidData, "Invalid position bytes")))?,
        );

        Ok((out, new_position))
    }

    pub fn write(&mut self, off: u32, pos: u64) -> Result<()> {
        // Check if there's enough space in the memory map
        if (self.mmap.len() as u64) < self.size + ENT_WIDTH {
            return Err(Box::new(Error::new(
                ErrorKind::UnexpectedEof,
                "Index is full",
            )));
        }

        // Write offset (u32) in big-endian format
        let off_bytes = off.to_be_bytes();
        self.mmap[self.size as usize..(self.size + OFF_WIDTH) as usize].copy_from_slice(&off_bytes);

        // Write position (u64) in big-endian format
        let pos_bytes = pos.to_be_bytes();
        self.mmap[(self.size + OFF_WIDTH) as usize..(self.size + ENT_WIDTH) as usize]
            .copy_from_slice(&pos_bytes);

        // Update the size
        self.size += ENT_WIDTH;

        Ok(())
    }
}
