use std::fs;
use std::sync::{Arc, Mutex};
use walrus::log::log::Log;
use walrus::log::config;
use walrus::log::segment::Record;

const TEST_BASE_DIR: &str = "/tmp/walrus_tests";

fn setup_test_env(test_name: &str) -> String {
    let test_dir = format!("{}/{}", TEST_BASE_DIR, test_name);
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    test_dir
}

fn cleanup_test_env(test_dir: &str) {
    let _ = fs::remove_dir_all(test_dir);
}

fn create_test_config(max_store_bytes: u64, max_index_bytes: u64) -> config::Config {
    config::Config {
        segment: config::InitSegment {
            max_store_bytes,
            max_index_bytes,
            initial_offset: 0,
        },
    }
}

#[test]
fn test_wal_basic_operations() {
    let test_dir = setup_test_env("basic_operations");
    let config = create_test_config(1024, 1024);
    
    // Create log
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Test append
    let mut record = Record::default();
    record.value = b"Hello, WAL!".to_vec();
    record.offset = 0;
    
    let append_result = log_guard.append(&mut record);
    assert!(append_result.is_ok(), "Append should succeed");
    let offset = append_result.unwrap();
    assert_eq!(offset, 0, "First record should have offset 0");
    
    // Test read
    let read_result = log_guard.read(0);
    assert!(read_result.is_ok(), "Read should succeed");
    let read_record = read_result.unwrap();
    assert_eq!(read_record.value, b"Hello, WAL!", "Read value should match");
    assert_eq!(read_record.offset, 0, "Read offset should match");
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_multiple_records() {
    let test_dir = setup_test_env("multiple_records");
    let config = create_test_config(1024, 1024);
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Append multiple records
    let test_messages = vec![
        "First message",
        "Second message", 
        "Third message",
        "Fourth message",
        "Fifth message"
    ];
    
    for (i, message) in test_messages.iter().enumerate() {
        let mut record = Record::default();
        record.value = message.as_bytes().to_vec();
        record.offset = i as u64;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
        let offset = append_result.unwrap();
        assert_eq!(offset, i as u64, "Record {} should have offset {}", i, i);
    }
    
    // Read all records back
    for (i, expected_message) in test_messages.iter().enumerate() {
        let read_result = log_guard.read(i as u64);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
        let read_record = read_result.unwrap();
        assert_eq!(read_record.value, expected_message.as_bytes(), "Read value {} should match", i);
        assert_eq!(read_record.offset, i as u64, "Read offset {} should match", i);
    }
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_segment_rotation() {
    let test_dir = setup_test_env("segment_rotation");
    // Use very small segment size to force rotation
    let config = create_test_config(100, 100);
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Append records until we force segment rotation
    for i in 0..10 {
        let mut record = Record::default();
        record.value = format!("Large message {} with enough bytes to exceed segment size limit", i).into_bytes();
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
        assert!(read_record.value.len() > 0, "Read value {} should not be empty", i);
    }
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_recovery() {
    let test_dir = setup_test_env("recovery");
    let config = create_test_config(1024, 1024);
    
    // First session: create log and append some records
    {
        let log_result = Log::new(test_dir.clone(), config.clone());
        assert!(log_result.is_ok(), "Log creation should succeed");
        
        let log = log_result.unwrap();
        let mut log_guard = log.lock().unwrap();
        
        for i in 0..5 {
            let mut record = Record::default();
            record.value = format!("Session 1 message {}", i).into_bytes();
            record.offset = i;
            
            let append_result = log_guard.append(&mut record);
            assert!(append_result.is_ok(), "Append {} should succeed", i);
        }
        
        drop(log_guard);
    }
    
    // Second session: recover log and append more records
    {
        let log_result = Log::new(test_dir.clone(), config);
        assert!(log_result.is_ok(), "Log recovery should succeed");
        
        let log = log_result.unwrap();
        let mut log_guard = log.lock().unwrap();
        
        // Verify existing records are still there
        for i in 0..5 {
            let read_result = log_guard.read(i);
            assert!(read_result.is_ok(), "Read {} should succeed", i);
            let read_record = read_result.unwrap();
            assert_eq!(read_record.value, format!("Session 1 message {}", i).into_bytes());
        }
        
        // Append new records
        for i in 5..10 {
            let mut record = Record::default();
            record.value = format!("Session 2 message {}", i).into_bytes();
            record.offset = i;
            
            let append_result = log_guard.append(&mut record);
            assert!(append_result.is_ok(), "Append {} should succeed", i);
        }
        
        // Verify all records
        for i in 0..10 {
            let read_result = log_guard.read(i);
            assert!(read_result.is_ok(), "Read {} should succeed", i);
            let read_record = read_result.unwrap();
            if i < 5 {
                assert_eq!(read_record.value, format!("Session 1 message {}", i).into_bytes());
            } else {
                assert_eq!(read_record.value, format!("Session 2 message {}", i).into_bytes());
            }
        }
        
        drop(log_guard);
    }
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_error_handling() {
    let test_dir = setup_test_env("error_handling");
    let config = create_test_config(1024, 1024);
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Try to read from non-existent offset
    let read_result = log_guard.read(999);
    assert!(read_result.is_err(), "Reading non-existent offset should fail");
    
    // Try to read from negative offset (should be handled gracefully)
    let read_result = log_guard.read(0);
    assert!(read_result.is_err(), "Reading from empty log should fail");
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_concurrent_access() {
    let test_dir = setup_test_env("concurrent_access");
    let config = create_test_config(1024, 1024);
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    
    // Test that multiple threads can access the log safely
    let log_clone = log.clone();
    let handle = std::thread::spawn(move || {
        let mut log_guard = log_clone.lock().unwrap();
        
        for i in 0..5 {
            let mut record = Record::default();
            record.value = format!("Thread 1 message {}", i).into_bytes();
            record.offset = i;
            
            let append_result = log_guard.append(&mut record);
            assert!(append_result.is_ok(), "Thread 1 append {} should succeed", i);
        }
    });
    
    // Main thread also accesses the log
    {
        let mut log_guard = log.lock().unwrap();
        
        for i in 0..5 {
            let mut record = Record::default();
            record.value = format!("Main thread message {}", i).into_bytes();
            record.offset = i + 5;
            
            let append_result = log_guard.append(&mut record);
            assert!(append_result.is_ok(), "Main thread append {} should succeed", i);
        }
    }
    
    handle.join().unwrap();
    
    // Verify all records were written
    let mut log_guard = log.lock().unwrap();
    for i in 0..10 {
        let read_result = log_guard.read(i);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
        let read_record = read_result.unwrap();
        assert_eq!(read_record.offset, i, "Read offset {} should match", i);
    }
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_large_data() {
    let test_dir = setup_test_env("large_data");
    let config = create_test_config(1024 * 1024, 1024 * 1024); // 1MB segments
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Test with larger data
    let large_message = "x".repeat(1000); // 1KB message
    
    for i in 0..10 {
        let mut record = Record::default();
        record.value = format!("{} - {}", large_message, i).into_bytes();
        record.offset = i;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
    }
    
    // Verify all records
    for i in 0..10 {
        let read_result = log_guard.read(i);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
        let read_record = read_result.unwrap();
        assert_eq!(read_record.value, format!("{} - {}", large_message, i).into_bytes());
    }
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}

#[test]
fn test_wal_cleanup() {
    let test_dir = setup_test_env("cleanup");
    let config = create_test_config(1024, 1024);
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    // Add some data
    for i in 0..5 {
        let mut record = Record::default();
        record.value = format!("Cleanup test message {}", i).into_bytes();
        record.offset = i;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
    }
    
    // Test close
    let close_result = log_guard.close();
    assert!(close_result.is_ok(), "Close should succeed");
    
    // Test remove
    let remove_result = log_guard.remove();
    assert!(remove_result.is_ok(), "Remove should succeed");
    
    // Verify directory is removed
    assert!(!std::path::Path::new(&test_dir).exists(), "Test directory should be removed");
}

#[test]
fn test_wal_performance() {
    let test_dir = setup_test_env("performance");
    let config = create_test_config(1024 * 1024, 1024 * 1024); // 1MB segments
    
    let log_result = Log::new(test_dir.clone(), config);
    assert!(log_result.is_ok(), "Log creation should succeed");
    
    let log = log_result.unwrap();
    let mut log_guard = log.lock().unwrap();
    
    let start_time = std::time::Instant::now();
    
    // Append many records quickly
    for i in 0..1000 {
        let mut record = Record::default();
        record.value = format!("Performance test message {}", i).into_bytes();
        record.offset = i;
        
        let append_result = log_guard.append(&mut record);
        assert!(append_result.is_ok(), "Append {} should succeed", i);
    }
    
    let append_time = start_time.elapsed();
    println!("Appended 1000 records in {:?}", append_time);
    
    // Read all records
    let read_start = std::time::Instant::now();
    for i in 0..1000 {
        let read_result = log_guard.read(i);
        assert!(read_result.is_ok(), "Read {} should succeed", i);
    }
    
    let read_time = read_start.elapsed();
    println!("Read 1000 records in {:?}", read_time);
    
    drop(log_guard);
    cleanup_test_env(&test_dir);
}
