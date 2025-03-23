use std::sync::{Arc, Mutex};

pub mod target_groups;
pub mod target_systems;
pub mod tasks;

pub type SharedMemory<T> = Arc<Mutex<T>>;
