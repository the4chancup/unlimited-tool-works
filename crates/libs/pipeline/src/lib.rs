//! The shared scaffolding every compiling tool (the Compiler, the Balls
//! compiler, the Stadium compiler) runs on: the run's memory budget and its cap
//! and cancellation, the rayon worker count, and the `CpkStem` every emitted
//! CPK name is validated against.

mod budget;
mod cpk_stem;
mod memory;
mod threads;

pub use budget::{Cancelled, MIN_FREE_BUDGET, MemoryBudget, Permit};
pub use cpk_stem::{CpkStem, CpkStemError};
pub use memory::memory_cap;
pub use threads::thread_count_detect;
