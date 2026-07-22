//! Infrastructure layer root: I/O adapters (SQLite, xlsx/PDF parsing, keychain,
//! IBGE fetch) that back the application and command layers.

pub mod btg_mapper;
pub mod btg_statement;
pub mod config_store;
pub mod db;
pub mod ibge;
pub mod secrets;
pub mod xlsx_parser;
