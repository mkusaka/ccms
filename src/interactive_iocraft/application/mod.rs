//! Application layer - Business logic and services

pub mod search_service;
pub mod session_service;
pub mod cache_service;
pub mod settings_service;
pub mod optimized_cache_service;

pub use search_service::SearchService;
pub use session_service::SessionService;
pub use cache_service::CacheService;
pub use settings_service::SettingsService;
pub use optimized_cache_service::{OptimizedCacheService, ThreadSafeOptimizedCache, CacheConfig};