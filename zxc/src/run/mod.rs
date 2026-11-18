//! Runtime lifecycle for the UI modules.
//!
//! `starter` builds the session, `runner` drives each UI socket event loop,
//! and `boundary` defines the traits and reply types shared by the handlers.
pub mod boundary;
pub mod runner;
pub mod starter;
