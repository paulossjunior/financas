//! Infrastructure layer root: I/O adapters (SQLite, xlsx/PDF parsing, keychain,
//! IBGE fetch) that back the application and command layers.

pub mod banestes_statement;
pub mod btg_mapper;
pub mod btg_statement;
pub mod config_store;
pub mod db;
pub mod ibge;
pub mod invoice_reader;
pub mod secrets;
pub mod statement_reader;
pub mod xlsx_parser;
