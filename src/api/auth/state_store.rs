use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct StateMetadata {
    pub inserted_at: Instant,
}

// Structure to store OAuth states with expiration
pub struct OAuthStateStore {
    states: Mutex<HashMap<String, StateMetadata>>,
    expiration: Duration,
}

impl OAuthStateStore {
    pub fn new(expiration: Duration) -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            expiration,
        }
    }

    pub fn add_state(&self, state: String) {
        let mut states = self.states.lock().unwrap();
        states.insert(
            state,
            StateMetadata {
                inserted_at: Instant::now(),
            },
        );

        // Clean up expired states
        states.retain(|_, metadata| metadata.inserted_at.elapsed() < self.expiration);
    }

    pub fn verify_and_remove_state(&self, state: &str) -> Option<StateMetadata> {
        let mut states = self.states.lock().unwrap();

        // Check if the state exists and is not expired
        if let Some(metadata) = states.get(state) {
            if metadata.inserted_at.elapsed() < self.expiration {
                let removed_state = states.remove(state);
                return Some(removed_state.unwrap());
            }
        }

        None
    }
}
