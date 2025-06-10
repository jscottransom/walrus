use byteorder::{BigEndian, ByteOrder};
use std::fs::File;

use std::io::{BufWriter, Read, Result, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LEN_WIDTH: usize = 8;
const BUFFER_CAPACITY: usize = 64 * 1024;
const FLUSH_THRESHOLD: usize = 64 * 1024; // 64 KB -> This supports frequent flushing
const SYNC_THRESHOLD: usize = 256 * 1024; // 256 KB -> Reduces the volume of sys calls.
const SYNC_INTERVAL: Duration = Duration::from_millis(100);

// The base object we will work with
// This will be wrapped in a mutex for safety during usage
// Simple wrapper around
pub struct Store {
    pub file: File,
    pub path: String,
    pub buf: BufWriter<File>,
    pub size: u64,
    last_sync: Instant,
    bytes_buffered: usize,
    sync_bytes: usize,
}

pub type SafeStore = Arc<Mutex<Store>>;

pub fn new(file: &File, path: String) -> Result<SafeStore> {
    let size = file.metadata()?.len() as u64;
    let file_obj = file.try_clone()?;

    let writer = BufWriter::new(file.try_clone().expect("clone failed"));
    Ok(Arc::new(Mutex::new(Store {
        file: file_obj,
        path: path,
        size: size,
        buf: writer,
        last_sync: Instant::now(),
        bytes_buffered: 0,
        sync_bytes: 0
    })))
}
impl Store {
    // Append a slice of bytes to the store log
    pub fn append(&mut self, p: &[u8]) -> Result<(u64, u64)> {
        
        let pos = self.size;

        // Write the length of the data
        let mut len_buf = [0u8; LEN_WIDTH];
        BigEndian::write_u64(&mut len_buf, p.len() as u64);
        self.buf.write_all(&len_buf)?;

        // Write the actual data
        self.buf.write_all(p)?;

        // Track the number of bytes written manually
        let written = p.len() + LEN_WIDTH;
        self.size += written as u64;

        // Flush any contents in the buffer
        // This pushes to from in-memory to OS Page Cache
        
        if self.buf.buffer().len() >= BUFFER_CAPACITY {
            
            self.buf.flush()?;

            // Sync data that exists in the buffer
            // Pushes from OS Page Cache to Disk
            self.file.sync_all()?;
        }

        // Return the number of written bytes and the position
        Ok((written as u64, pos))
    }

    pub fn read(&mut self, pos: u64) -> Result<Vec<u8>> {

        // Flush any contents in the buffer
        self.buf.flush()?;
        let mut size = vec![0u8; LEN_WIDTH];

        // Start reading from the given position
        self.file.seek(SeekFrom::Start(pos))?;
        self.file.read_exact(&mut size)?;

        // Encode size
        let new_pos = u64::from_be_bytes(size.try_into().unwrap());
        let mut b = vec![0u8; new_pos as usize];

        // Read the actual bytes
        self.file.seek(SeekFrom::Start(pos + LEN_WIDTH as u64))?;
        self.file.read_exact(&mut b)?;
        Ok(b)
    }

    // Reads len(p) bytes into p, beginning at the offset in the
    // store file.
    pub fn read_at(&mut self, p: &mut [u8], off: u64) -> Result<usize> {
        self.buf.flush()?;
        self.file.seek(SeekFrom::Start(off))?;
        self.file.read_exact(p)?;

        Ok(p.len())
    }

    pub fn close(&mut self) -> Result<()> {
        self.buf.flush()?;
        
        Ok(())
    }
}
