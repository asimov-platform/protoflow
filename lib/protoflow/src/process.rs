// This is free and unencumbered software released into the public domain.

pub type ProcessID = usize;

pub trait Process {
    fn id(&self) -> ProcessID;
    fn is_alive(&self) -> bool;
}