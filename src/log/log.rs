use byteorder::{BigEndian, ByteOrder};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::io;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::str::FromStr;
use super::{config, segment, store};

// Custom Result type for the log operations
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Abstraction around the Log that will be the entry point for writing to the Log
// And committing changes
pub struct Log {
    pub dir: String,
    pub config: config::Config,
    pub active_segment: Option<segment::Segment>,
    pub segments: Vec<segment::Segment>,
}

pub type SafeLog = Arc<Mutex<Log>>;

fn setup_log(dir: String) -> Result<Vec<u64>> {
    // Read through the given directory
    let mut base_offsets: Vec<u64> = Vec::new();
    let dir_path = Path::new(&dir);
    
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            
            // Check if the file has an extension
            if let Some(ext) = Path::new(&file_name_str.to_string()).extension() {
                if let Some(ext_str) = ext.to_str() {
                    if ext_str == "store" {
                        // Extract the base offset from the filename
                        let stem = file_name_str.trim_end_matches(".store");
                        if let Ok(offset) = u64::from_str(stem) {
                            base_offsets.push(offset);
                        }
                    }
                }
            }
        }
    }

    // Sort the offsets
    base_offsets.sort();
    Ok(base_offsets)
}

fn new_log(dir: String, config: config::Config) -> Result<SafeLog> {
    // Set default values if not provided
    let mut config = config;
    if config.segment.max_store_bytes == 0 {
        config.segment.max_store_bytes = 1024;
    }

    if config.segment.max_index_bytes == 0 {
        config.segment.max_index_bytes = 1024;
    }

    // Create the log directory if it doesn't exist
    fs::create_dir_all(&dir)?;

    // Setup existing segments
    let base_offsets = setup_log(dir.clone())?;
    let mut segments = Vec::new();
    let mut active_segment = None;

    // Load existing segments
    for &offset in &base_offsets {
        let segment = segment::new(&dir, format!("{}/{}", dir, offset), offset, config.clone())
            .map_err(|e| e)?;
        segments.push(segment);
    }

    // Set the last segment as active if segments exist
    if let Some(mut last_segment) = segments.pop() {
        active_segment = Some(last_segment);
    }

    let log = Log {
        dir,
        config,
        active_segment,
        segments,
    };

    Ok(Arc::new(Mutex::new(log)))
}

impl Log {
    pub fn new(dir: String, config: config::Config) -> Result<SafeLog> {
        new_log(dir, config)
    }

    pub fn append(&mut self, record: &mut segment::Record) -> Result<u64> {
        // If no active segment or current segment is full, create a new one
        if self.active_segment.is_none() || self.active_segment.as_mut().unwrap().is_maxed() {
            self.new_segment()?;
        }

        // Append to the active segment
        if let Some(ref mut segment) = self.active_segment {
            return segment.append(record);
        }

        Err("No active segment available".into())
    }

    pub fn read(&mut self, offset: u64) -> Result<segment::Record> {
        // Find the segment that contains this offset
        for segment in &mut self.segments {
            if offset >= segment.base_offset() && offset < segment.next_offset() {
                let segment_result = segment.read(offset);
                return segment_result;
            }
        }

        // Check active segment
        if let Some(ref mut segment) = self.active_segment {
            if offset >= segment.base_offset() && offset < segment.next_offset() {
                let segment_result = segment.read(offset);
                return segment_result;
            }
        }

        Err("Offset not found in any segment".into())
    }

    fn new_segment(&mut self) -> Result<()> {
        let base_offset = if let Some(ref segment) = self.active_segment {
            segment.next_offset()
        } else {
            self.config.segment.initial_offset
        };

        // Move current active segment to segments list
        if let Some(segment) = self.active_segment.take() {
            self.segments.push(segment);
        }

        // Create new active segment
        let new_segment = segment::new(&self.dir, format!("{}/{}", self.dir, base_offset), base_offset, self.config.clone())?;
        self.active_segment = Some(new_segment);
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        // Close active segment
        if let Some(ref mut segment) = self.active_segment {
            segment.close()?;
        }

        // Close all segments
        for segment in &mut self.segments {
            segment.close()?;
        }

        Ok(())
    }

    pub fn remove(&mut self) -> Result<()> {
        // Remove active segment
        if let Some(ref mut segment) = self.active_segment {
            segment.remove()?;
        }

        // Remove all segments
        for segment in &mut self.segments {
            segment.remove()?;
        }

        // Remove the directory
        fs::remove_dir_all(&self.dir)?;

        Ok(())
    }
}