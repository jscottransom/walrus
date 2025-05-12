use std::io::{ErrorKind, Result};
use std::fs::File;
use memmap2::MmapMut;


const OFF_WIDTH: u64 = 4;
const POS_WIDTH: u64 = 8;
const ENT_WIDTH: u64 = OFF_WIDTH + POS_WIDTH;

struct Index {
    file: File,
    mmap: MmapMut,
    size: u64,
}

impl Index {
    fn new_index(file: &File, config: Config) -> Result<Self> {
        let size = file.metadata()?.len() as u64;
        let file_obj = file.try_clone()?;
        file_obj.set_len(&c.Segment.Max_Index_Bytes as i64)?;
        let mut mmap = unsafe { MmapMut::map_mut(&file_obj)? };
        
        let index = Index{
            file: file_obj,
            mmap: mmap,
            size: size
        };
        Ok(index)
    }

    fn close(&mut self) -> Result<()> {
        self.file.sync_all()?;
        self.file.set_len(self.size);

        Ok(())

    }

    fn read(&self, offset: i64) -> Result<(u32, u64)> {
        
    
        if self.size == 0 {
            return Ok((0 as u32, 0 as u64));
        } 
        
        let index: u32 = if offset == -1 {
            (self.size as u32 / ENT_WIDTH as u32) - 1
        } else {
            offset as u32
        };

        let position = (index as u64) * ENT_WIDTH;
        if self.size < position + ENT_WIDTH {
            return Ok((0 as u32, 0 as u64));
        } 
          
        let out_slice: [u8; 4] = self.mmap[position as usize..(position + OFF_WIDTH) as usize].try_into().expect("Expected 4 bytes");
        let out = u32::from_be_bytes(out_slice);
    
        let new_slice: [u8; 8] = self.mmap[(position + OFF_WIDTH) as usize..(position + ENT_WIDTH) as usize].try_into().expect("Expected 8 bytes");
        let new_position = u64::from_be_bytes(new_slice);

        Ok((out, new_position))

    }

    fn write(&mut self, off: u32, pos: u64) -> Result<()> {
        // Check if there's enough space in the memory map
        if (self.mmap.len() as u64) < self.size + ENT_WIDTH {
            return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "Not enough space in memory"));
        }

        // Writing the offset (u32) into mmap
        let off_bytes = off.to_be_bytes();
        self.mmap[self.size as usize..(self.size + OFF_WIDTH) as usize].copy_from_slice(&off_bytes);

        
        let pos_bytes = pos.to_be_bytes();  // Convert pos (u64) to bytes in big-endian order
        self.mmap[(self.size + OFF_WIDTH) as usize..(self.size + ENT_WIDTH) as usize].copy_from_slice(&pos_bytes);

        // Update the size
        self.size += ENT_WIDTH;

        Ok(())
    }
}