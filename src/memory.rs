use std::sync::{Arc, Mutex};

pub mod groups;
pub mod systems;
pub mod tasks;

pub type SharedMemory<T> = Arc<Mutex<T>>;
