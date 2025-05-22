use std::{io, os};
use super::store;

pub struct Segment {
    store: store::Store
}