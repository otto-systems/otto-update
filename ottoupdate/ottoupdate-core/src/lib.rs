pub mod applier;
pub mod backup;
pub mod coordinator;
pub mod decision;
pub mod device;
pub mod downloader;
pub mod manifest_fetcher;
pub mod policy;
pub mod repo;
pub mod schema;
pub mod state_machine;
pub mod traits;
pub mod verifier;

#[cfg(test)]
pub mod mocks;

pub mod prelude {
    pub type Result<T> = std::result::Result<T, anyhow::Error>;
}
