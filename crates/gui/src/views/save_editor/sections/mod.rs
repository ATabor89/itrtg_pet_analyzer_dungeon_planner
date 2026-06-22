//! Structured editor sections. Each renders one logically-grouped slice of the
//! save, reading and writing through the shared [`EditSession`]. The raw tree
//! navigator lives here too as a special "section".

pub mod adventure;
pub mod bulk;
pub mod campaigns;
pub mod challenges;
pub mod dungeons;
pub mod equip_builder;
pub mod equipment;
pub mod fishing;
pub mod gems;
pub mod inventory;
pub mod pets;
pub mod planet;
pub mod progression;
pub mod raw_tree;
pub mod resources;
pub mod stats;
pub mod village;
