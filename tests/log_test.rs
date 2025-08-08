use std::fs;
use walrus::log::log::Log;
use walrus::log::config;
use walrus::log::segment::Record;

#[test]
fn test_log_creation() {
    let test_dir = "/tmp/test_log_creation";
    
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(test_dir);
    
    let config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 1024,
            max_index_bytes: 1024,
            initial_offset: 0,
        },
    };
    
    let log_result = Log::new(test_dir.to_string(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    // Clean up
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_log_append_and_read() {
    let test_dir = "/tmp/test_log_append_read";
    
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(test_dir);
    
    let config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 1024,
            max_index_bytes: 1024,
            initial_offset: 0,
        },
    };
    
    let log_result = Log::new(test_dir.to_string(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Create a test record
    let mut record = Record::default();
    record.value = b"test message".to_vec();
    record.offset = 0;
    
    // Append the record
    let append_result = log_guard.append(&mut record);
    assert!(append_result.is_ok(), "Append should succeed");
    let offset = append_result.unwrap();
    assert_eq!(offset, 0, "First record should have offset 0");
    
    // Read the record back
    let read_result = log_guard.read(0);
    assert!(read_result.is_ok(), "Read should succeed");
    let read_record = read_result.unwrap();
    assert_eq!(read_record.value, b"test message", "Read value should match written value");
    assert_eq!(read_record.offset, 0, "Read offset should match written offset");
    
    // Clean up
    drop(log_guard);
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_log_multiple_records() {
    let test_dir = "/tmp/test_log_multiple";
    
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(test_dir);
    
    let config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 1024,
            max_index_bytes: 1024,
            initial_offset: 0,
        },
    };
    
    let log_result = Log::new(test_dir.to_string(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let mut log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Append multiple records
    for i in 0..5 {
        let mut record = Record::default();
        record.value = format!("message {}", i).into_bytes();
        record.offset = i;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
        let offset = append_result.unwrap();
        assert_eq!(offset, i, "Record {} should have offset {}", i, i);
    }
    
    // Read all records back
    for i in 0..5 {
        let read_result = log_guard.read(i);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
        let read_record = read_result.unwrap();
        assert_eq!(read_record.value, format!("message {}", i).into_bytes(), "Read value {} should match", i);
        assert_eq!(read_record.offset, i, "Read offset {} should match", i);
    }
    
    // Clean up
    drop(log_guard);
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_log_segment_rotation() {
    let test_dir = "/tmp/test_log_rotation";
    
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(test_dir);
    
    // Use very small segment size to force rotation
    let config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 100, // Very small to force rotation
            max_index_bytes: 100,
            initial_offset: 0,
        },
    };
    
    let log_result = Log::new(test_dir.to_string(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let mut log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Append records until we force segment rotation
    for i in 0..10 {
        let mut record = Record::default();
        record.value = format!("large message {} with enough bytes to exceed segment size", i).into_bytes();
        record.offset = i;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
    }
    
    // Verify we can still read records from different segments
    for i in 0..10 {
        let read_result = log_guard.read(i);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
        let read_record = read_result.unwrap();
        assert_eq!(read_record.offset, i, "Read offset {} should match", i);
    }
    
    // Clean up
    drop(log_guard);
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_log_invalid_offset() {
    let test_dir = "/tmp/test_log_invalid";
    
    // Clean up any existing test directory
    let _ = fs::remove_dir_all(test_dir);
    
    let config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 1024,
            max_index_bytes: 1024,
            initial_offset: 0,
        },
    };
    
    let log_result = Log::new(test_dir.to_string(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let mut log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Try to read from non-existent offset
    let read_result = log_guard.read(999);
    assert!(read_result.is_err(), "Reading non-existent offset should fail");
    
    // Clean up
    drop(log_guard);
    let _ = fs::remove_dir_all(test_dir);
}
