// Defaults for initializing segments
pub struct InitSegment {
    pub max_store_bytes: u64,
    pub max_index_bytes: u64,
    pub initial_offset: u64,
}

// Configuration object for handling segments
pub struct Config {
    pub segment: InitSegment,
}
