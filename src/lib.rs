#[macro_use]
extern crate log;

mod bot;
mod config;
mod db;
mod lichess;
mod models;
mod run;
mod web;

pub use run::run;
