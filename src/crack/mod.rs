//! Password cracking engine and related types.
//!
//! Designed so new file formats can be added by implementing [`handlers::PasswordHandler`].

pub mod charset;
pub mod engine;
pub mod handlers;
pub mod types;

#[cfg(test)]
mod tests_engine;
#[cfg(test)]
mod tests_six;
#[cfg(test)]
mod tests_pdf;
#[cfg(test)]
mod tests_archive;
#[cfg(test)]
mod tests_office;
#[cfg(test)]
mod tests_matrix;

pub use charset::CharsetOptions;
pub use engine::{CrackEngine, CrackProgress, EngineControl};
pub use types::{CrackJob, FileKind, JobStatus, TargetFile};
