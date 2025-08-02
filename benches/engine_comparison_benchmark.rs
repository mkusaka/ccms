use ccms::{RayonEngine, SearchEngineTrait, SearchOptions, SmolEngine, parse_query};
use codspeed_criterion_compat::{
    BenchmarkId, Criterion, black_box, criterion_group, criterion_main,
};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

struct TestEnvironment {
    _temp_dir: TempDir,
}

impl TestEnvironment {
    fn new(num_files: usize, lines_per_file: usize) -> Self {
        let temp_dir = TempDir::new().unwrap();

        for file_idx in 0..num_files {
            let file_path = temp_dir.path().join(format!("session_{file_idx}.jsonl"));
            let mut file = File::create(&file_path).unwrap();

            for line_idx in 0..lines_per_file {
                // Create realistic session data with various message types
                let (msg_type, content) = match line_idx % 5 {
                    0 => (
                        "user",
                        format!(
                            "I need help with implementing feature {line_idx}. Can you show me how to handle error code {line_idx}?"
                        ),
                    ),
                    1 => (
                        "assistant",
                        format!(
                            "I'll help you implement feature {line_idx}. Here's a solution for error code {line_idx}: First, let's understand the problem..."
                        ),
                    ),
                    2 => (
                        "user",
                        format!(
                            "Testing the implementation. Debug log shows: process {line_idx} failed with status {}",
                            line_idx % 10
                        ),
                    ),
                    3 => (
                        "assistant",
                        format!(
                            "Looking at the debug log for process {line_idx}, the issue seems to be related to memory allocation. Let me analyze..."
                        ),
                    ),
                    _ => (
                        "user",
                        format!(
                            "Thanks! Now optimizing performance for algorithm {line_idx}. Current benchmark: {}ms",
                            line_idx * 10
                        ),
                    ),
                };

                writeln!(
                    file,
                    r#"{{"type":"{msg_type}","message":{{"role":"{msg_type}","content":"{content}","isTruncated":false}},"uuid":"{file_idx}-{line_idx}","timestamp":"2024-01-{:02}T{:02}:00:{:02}Z","sessionId":"session{file_idx}","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/Users/dev/project{file_idx}","version":"1.0"}}"#,
                    (line_idx % 28) + 1, line_idx % 24, line_idx % 60
                ).unwrap();
            }
        }

        TestEnvironment {
            _temp_dir: temp_dir,
        }
    }
}

fn benchmark_engine_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_comparison");

    // Test different workload sizes
    let workloads = vec![
        ("small", 5, 100),    // 5 files, 100 lines each = 500 messages
        ("medium", 10, 500),  // 10 files, 500 lines each = 5,000 messages
        ("large", 20, 1000),  // 20 files, 1000 lines each = 20,000 messages
        ("xlarge", 50, 1000), // 50 files, 1000 lines each = 50,000 messages
    ];

    for (size_name, num_files, lines_per_file) in workloads {
        let env = TestEnvironment::new(num_files, lines_per_file);
        let pattern = format!("{}/*.jsonl", env._temp_dir.path().display());

        // Test different query types
        let queries = vec![
            ("simple", "error"),
            ("phrase", r#""error code""#),
            ("complex", "error AND (code OR debug)"),
            ("not", "NOT failed"),
            ("regex", r#"/process \d+/"#),
        ];

        for (query_name, query_str) in queries {
            let query = parse_query(query_str).unwrap();
            let options = SearchOptions::default();

            // Benchmark Smol engine
            group.bench_with_input(
                BenchmarkId::new(format!("smol/{query_name}"), size_name),
                &(&pattern, &query, &options),
                |b, (pattern, query, options)| {
                    b.iter(|| {
                        let engine = SmolEngine::new((*options).clone());
                        let (results, _, _) =
                            engine.search(pattern, black_box((*query).clone())).unwrap();
                        black_box(results.len())
                    });
                },
            );

            // Benchmark Rayon engine
            group.bench_with_input(
                BenchmarkId::new(format!("rayon/{query_name}"), size_name),
                &(&pattern, &query, &options),
                |b, (pattern, query, options)| {
                    b.iter(|| {
                        let engine = RayonEngine::new((*options).clone());
                        let (results, _, _) =
                            engine.search(pattern, black_box((*query).clone())).unwrap();
                        black_box(results.len())
                    });
                },
            );
        }

        // Test with filters
        let filtered_query = parse_query("error").unwrap();
        let filtered_options = SearchOptions {
            role: Some("user".to_string()),
            max_results: Some(100),
            ..Default::default()
        };

        group.bench_with_input(
            BenchmarkId::new("smol/filtered", size_name),
            &(&pattern, &filtered_query, &filtered_options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = SmolEngine::new((*options).clone());
                    let (results, _, _) =
                        engine.search(pattern, black_box((*query).clone())).unwrap();
                    black_box(results.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rayon/filtered", size_name),
            &(&pattern, &filtered_query, &filtered_options),
            |b, (pattern, query, options)| {
                b.iter(|| {
                    let engine = RayonEngine::new((*options).clone());
                    let (results, _, _) =
                        engine.search(pattern, black_box((*query).clone())).unwrap();
                    black_box(results.len())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_engine_comparison);
criterion_main!(benches);
