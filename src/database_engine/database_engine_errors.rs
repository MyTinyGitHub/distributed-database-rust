use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseEngineError {
    #[error("lexer: {0}")]
    Wal(String),
}
