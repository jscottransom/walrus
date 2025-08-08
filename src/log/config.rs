// Defaults for initializing segments
#[derive(Clone)]
pub struct InitSegment {
    pub max_store_bytes: u64,
    pub max_index_bytes: u64,
    pub initial_offset: u64,
}

// Configuration object for handling segments
#[derive(Clone)]
pub struct Config {
    pub segment: InitSegment,
}
