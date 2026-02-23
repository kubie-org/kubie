use std::fs::DirBuilder;
use std::{collections::HashMap, panic::UnwindSafe};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::ioutil;

pub mod paths {
    use std::path::{Path, PathBuf};

    use lazy_static::lazy_static;

    lazy_static! {
        static ref KUBIE_DATA_DIR: PathBuf = {
            let base_data_dir = dirs::data_local_dir().expect("Could not get local data dir");
            base_data_dir.join("kubie")
        };
        static ref KUBIE_STATE_PATH: PathBuf = KUBIE_DATA_DIR.join("state.json");
        static ref KUBIE_STATE_LOCK_PATH: PathBuf = KUBIE_DATA_DIR.join(".state.json.lock");
    }

    #[inline]
    pub fn data_dir() -> &'static Path {
        &KUBIE_DATA_DIR
    }

    #[inline]
    pub fn state() -> &'static Path {
        &KUBIE_STATE_PATH
    }

    #[inline]
    pub fn state_lock() -> &'static Path {
        &KUBIE_STATE_LOCK_PATH
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct State {
    /// The most recently entered context across all sessions.
    /// Only updated when `track_last_used` config option is enabled.
    pub last_context: Option<String>,

    /// This map stores the last namespace in which a context was used, in order to restore the namespace
    /// when the context is entered again.
    ///
    /// The key represents the name of the context and the value is the namespace's name.
    pub namespace_history: HashMap<String, Option<String>>,
}

impl State {
    /// Loads the state.json from the filesystem, waiting for a file lock to ensure no other
    /// concurrent Kubie processes are accessing/writing the file at the same time.
    pub fn load() -> Result<State> {
        Self::access(Ok)
    }

    /// Takes a closure that allows for modifications of the state. Automatically handles
    /// locking/unlocking and saving after execution of the closure.
    pub fn modify<F: FnOnce(&mut State) -> Result<()> + UnwindSafe>(func: F) -> Result<()> {
        Self::access(|mut state| {
            func(&mut state)?;
            state.save()?;
            Ok(())
        })
    }

    fn access<R, F: FnOnce(State) -> Result<R> + UnwindSafe>(func: F) -> Result<R> {
        // Create directory where state and lock will live.
        DirBuilder::new()
            .recursive(true)
            .create(paths::data_dir())
            .with_context(|| format!("Could not create data dir: {}", paths::data_dir().display()))?;

        ioutil::file_lock(paths::state_lock(), || {
            State::read_and_parse()
                .with_context(|| format!("Could not load state file: {}", paths::state().display()))
                .and_then(func)
        })
    }

    fn read_and_parse() -> Result<State> {
        if !paths::state().exists() {
            return Ok(State::default());
        }
        ioutil::read_json(paths::state())
            .with_context(|| format!("Failed to read state from '{}'", paths::state().display()))
    }

    fn save(&self) -> Result<()> {
        ioutil::write_json(paths::state(), self)
            .with_context(|| format!("Failed to write state to '{}'", paths::state().display()))
    }
}
