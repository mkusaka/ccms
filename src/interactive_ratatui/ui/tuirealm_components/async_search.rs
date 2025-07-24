use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::sync::Arc;

use crate::interactive_ratatui::application::search_service::SearchService;
use crate::interactive_ratatui::domain::models::{SearchRequest, SearchResponse};

/// Async search handler for tui-realm
pub struct AsyncSearchHandler {
    search_sender: Option<Sender<SearchRequest>>,
    result_receiver: Option<Receiver<SearchResponse>>,
    search_service: Arc<SearchService>,
}

impl AsyncSearchHandler {
    pub fn new(search_service: Arc<SearchService>) -> Self {
        Self {
            search_sender: None,
            result_receiver: None,
            search_service,
        }
    }

    /// Start the search worker thread
    pub fn start(&mut self) {
        let (req_tx, req_rx) = channel::<SearchRequest>();
        let (res_tx, res_rx) = channel::<SearchResponse>();
        
        let search_service = self.search_service.clone();
        
        // Spawn worker thread
        thread::spawn(move || {
            while let Ok(request) = req_rx.recv() {
                match search_service.search(request.clone()) {
                    Ok(response) => {
                        if res_tx.send(response).is_err() {
                            break; // Main thread has disconnected
                        }
                    }
                    Err(_e) => {
                        // Send error response
                        let error_response = SearchResponse {
                            id: request.id,
                            results: vec![],
                        };
                        let _ = res_tx.send(error_response);
                    }
                }
            }
        });

        self.search_sender = Some(req_tx);
        self.result_receiver = Some(res_rx);
    }

    /// Send a search request
    pub fn search(&self, request: SearchRequest) -> Result<(), String> {
        if let Some(sender) = &self.search_sender {
            sender.send(request).map_err(|e| e.to_string())
        } else {
            Err("Search worker not started".to_string())
        }
    }

    /// Check for search results (non-blocking)
    pub fn poll_results(&self) -> Option<SearchResponse> {
        if let Some(receiver) = &self.result_receiver {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
}

/// Debouncer for search requests
pub struct SearchDebouncer {
    delay_ms: u64,
    last_input_time: Option<std::time::Instant>,
    pending_search: Option<String>,
}

impl SearchDebouncer {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay_ms,
            last_input_time: None,
            pending_search: None,
        }
    }

    /// Update query and check if search should be triggered
    pub fn update_query(&mut self, query: String) -> bool {
        self.last_input_time = Some(std::time::Instant::now());
        self.pending_search = Some(query);
        false // Don't trigger immediately
    }

    /// Check if enough time has passed to trigger search
    pub fn should_search(&mut self) -> Option<String> {
        if let Some(last_time) = self.last_input_time {
            if last_time.elapsed().as_millis() >= self.delay_ms as u128 {
                let query = self.pending_search.take();
                self.last_input_time = None;
                return query;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_debouncer() {
        let mut debouncer = SearchDebouncer::new(100);
        
        // Update query
        assert!(!debouncer.update_query("test".to_string()));
        assert!(debouncer.should_search().is_none());
        
        // Wait for debounce delay
        thread::sleep(Duration::from_millis(150));
        assert_eq!(debouncer.should_search(), Some("test".to_string()));
        
        // After search, should return None
        assert!(debouncer.should_search().is_none());
    }
}