//! app_core - AI Companion Core Engine
//!
//! Shared Core containing memory management, cognitive architecture,
//! FSRS dynamics, ASE proactive engine, and retrieval pipelines.

pub mod error;
pub mod models;
pub mod storage;
pub mod embedding;
pub mod retrieval;

pub use error::{AppError, AppResult};
