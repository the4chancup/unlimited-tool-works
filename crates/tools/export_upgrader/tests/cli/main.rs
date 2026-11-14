//! The Export upgrader's command line run in-process
//! (`spec/export_upgrader.md` "CLI"): the refusals, the findings and the files
//! written, observed as the exit code, the `Message` events the tool emits
//! (which the binary prints one line each) and the folders on disk.

#[path = "../common/mod.rs"]
mod common;

mod carried;
mod cli;
mod input;
mod kits;
mod loose;
mod players;
mod referees;
mod report;
mod roots;
mod settings;
