//! Structured editor sections. Each renders one logically-grouped slice of the
//! save, reading and writing through the shared [`EditSession`]. The raw tree
//! navigator lives here too as a special "section".

pub mod bulk;
pub mod equip_builder;
pub mod equipment;
pub mod inventory;
pub mod pets;
pub mod raw_tree;
pub mod resources;
