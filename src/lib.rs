//! anirel - find the related anime you have not watched yet.

pub mod app;
pub mod art;
pub mod cli;
pub mod commands;
pub mod config;
pub mod domain;
pub mod engine;
pub mod error;
pub mod infra;
pub mod output;
pub mod sys;
pub mod tui;
pub mod ui;

pub use error::{AnirelError, Result};
