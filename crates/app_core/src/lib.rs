//! app_core - AI Companion Core Engine
//!
//! Shared Core containing memory management, cognitive architecture,
//! FSRS dynamics, ASE proactive engine, and retrieval pipelines.

pub mod consolidation;
pub mod dynamics;
pub mod embedding;
pub mod error;
pub mod models;
pub mod retrieval;
pub mod storage;

pub use error::{AppError, AppResult};
