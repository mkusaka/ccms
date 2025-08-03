use ccms::search::file_discovery::discover_claude_files;
use ccms::utils::path_encoding::encode_project_path;
use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

fn benchmark_glob_pattern_approach(c: &mut Criterion) {
    c.bench_function("glob_pattern_approach", |b| {
        b.iter(|| {
            let project_path = "/Users/masatomokusaka/src/github.com/mkusaka/ccms";

            // Convert to absolute path
            let absolute_path = if Path::new(project_path).is_absolute() {
                project_path.to_string()
            } else {
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| cwd.join(project_path).canonicalize().ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| project_path.to_string())
            };

            let encoded_path = encode_project_path(&absolute_path);
            let claude_project_dir = format!("~/.claude/projects/{encoded_path}*/**/*.jsonl");

            let _ = discover_claude_files(Some(&claude_project_dir));
        })
    });
}

// Test different project paths
fn benchmark_project_paths(c: &mut Criterion) {
    let test_paths = vec![
        ("/", "root_path"),
        ("/Users/masatomokusaka", "user_home"),
        (
            "/Users/masatomokusaka/src/github.com/mkusaka/ccms",
            "specific_project",
        ),
        (".", "current_dir"),
    ];

    for (path, name) in test_paths {
        c.bench_function(&format!("glob_pattern_{}", name), |b| {
            b.iter(|| {
                let absolute_path = if Path::new(path).is_absolute() {
                    path.to_string()
                } else {
                    std::env::current_dir()
                        .ok()
                        .and_then(|cwd| cwd.join(path).canonicalize().ok())
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.to_string())
                };

                let encoded_path = encode_project_path(&absolute_path);
                let claude_project_dir = format!("~/.claude/projects/{encoded_path}*/**/*.jsonl");

                let _ = discover_claude_files(Some(&claude_project_dir));
            })
        });
    }
}

criterion_group!(
    benches,
    benchmark_glob_pattern_approach,
    benchmark_project_paths
);
criterion_main!(benches);
