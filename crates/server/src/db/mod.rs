//! Database layer: pool initialization, migrations, repository trait + SQLite impl,
//! WAL checkpoint + backup.

pub mod backup;
pub mod pool;
pub mod repository;

pub use pool::init_pool;
pub use repository::{Repository, SqliteRepository};
