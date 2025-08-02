pub mod engine;
pub mod file_discovery;
pub mod rayon_engine;
pub mod rayon_limited_engine;
pub mod smol_engine;

pub use engine::{SearchEngineTrait, format_search_result};
pub use file_discovery::{default_claude_pattern, discover_claude_files, expand_tilde};
pub use rayon_engine::RayonEngine;
pub use rayon_limited_engine::RayonLimitedEngine;
pub use smol_engine::SmolEngine;
