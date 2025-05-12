use std::io::Result;
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
        file_obj.set_len(&c.Segment.Max_Index_Bytes)?;
        let mut mmap = unsafe { MmapMut::map_mut(&file_obj)? };
        
        let index = Index{
            file: file_obj,
            mmap: mmap,
            size: size
        };
        Ok(index)
    }
}