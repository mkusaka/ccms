use ccms::{SearchEngine, SearchOptions, SessionMessage, parse_query};
use codspeed_criterion_compat::{
    BenchmarkId, Criterion, black_box, criterion_group, criterion_main,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

// Benchmark individual components to identify bottlenecks

fn create_test_jsonl(num_lines: usize, line_size: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.jsonl");
    let mut file = File::create(&test_file).unwrap();

    let long_text = "x".repeat(line_size);

    for i in 0..num_lines {
        writeln!(
            file,
            r#"{{"type":"user","message":{{"role":"user","content":"Message {} test content {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
            i,
            long_text,
            i,
            i % 60
        )
        .unwrap();
    }

    (temp_dir, test_file.to_string_lossy().to_string())
}

fn benchmark_file_reading(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_reading");

    for size in [100, 1000, 10000].iter() {
        let (_temp_dir, test_file) = create_test_jsonl(*size, 100);

        group.bench_with_input(
            BenchmarkId::new("std_read", size),
            &test_file,
            |b, test_file| {
                b.iter(|| {
                    let content = fs::read_to_string(test_file).unwrap();
                    black_box(content.len())
                });
            },
        );
    }
    group.finish();
}

fn benchmark_json_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parsing");

    let simple_json = r#"{"type":"user","message":{"role":"user","content":"Test message"},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#;

    let complex_json = r#"{"type":"assistant","message":{"id":"msg1","type":"message","role":"assistant","model":"claude-3","content":[{"type":"text","text":"Hello world"},{"type":"tool_use","id":"tool1","name":"Bash","input":{"command":"ls -la"}}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}},"uuid":"456","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"123","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","requestId":"req1"}"#;

    group.bench_function("simd_json_simple", |b| {
        b.iter(|| {
            let mut bytes = simple_json.as_bytes().to_vec();
            let msg: SessionMessage = simd_json::serde::from_slice(&mut bytes).unwrap();
            black_box(msg)
        });
    });

    group.bench_function("simd_json_complex", |b| {
        b.iter(|| {
            let mut bytes = complex_json.as_bytes().to_vec();
            let msg: SessionMessage = simd_json::serde::from_slice(&mut bytes).unwrap();
            black_box(msg)
        });
    });

    group.bench_function("serde_json_simple", |b| {
        b.iter(|| {
            let msg: SessionMessage = serde_json::from_str(simple_json).unwrap();
            black_box(msg)
        });
    });

    group.bench_function("serde_json_complex", |b| {
        b.iter(|| {
            let msg: SessionMessage = serde_json::from_str(complex_json).unwrap();
            black_box(msg)
        });
    });

    group.finish();
}

fn benchmark_search_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_matching");

    let short_text = "This is a short test message";
    let long_text = "x".repeat(10000) + " test content " + &"y".repeat(10000);

    // Literal search
    group.bench_function("literal_short", |b| {
        b.iter(|| black_box(short_text.contains("test")));
    });

    group.bench_function("literal_long", |b| {
        b.iter(|| black_box(long_text.contains("test")));
    });

    // Regex search
    let regex = regex::Regex::new(r"test.*content").unwrap();

    group.bench_function("regex_short", |b| {
        b.iter(|| black_box(regex.is_match(short_text)));
    });

    group.bench_function("regex_long", |b| {
        b.iter(|| black_box(regex.is_match(&long_text)));
    });

    group.finish();
}

fn benchmark_content_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_extraction");

    let simple_msg: SessionMessage = serde_json::from_str(
        r#"{"type":"user","message":{"role":"user","content":"Test message"},"uuid":"123","timestamp":"2024-01-01T00:00:00Z","sessionId":"s1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}"#
    ).unwrap();

    let complex_msg: SessionMessage = serde_json::from_str(
        r#"{"type":"assistant","message":{"id":"msg1","type":"message","role":"assistant","model":"claude-3","content":[{"type":"text","text":"Hello world"},{"type":"tool_use","id":"tool1","name":"Bash","input":{"command":"ls -la"}},{"type":"tool_result","tool_use_id":"tool1","content":"file1\nfile2\nfile3"}],"stop_reason":"tool_use","stop_sequence":null,"usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}},"uuid":"456","timestamp":"2024-01-01T00:00:01Z","sessionId":"s1","parentUuid":"123","isSidechain":false,"userType":"external","cwd":"/test","version":"1.0","requestId":"req1"}"#
    ).unwrap();

    group.bench_function("extract_simple", |b| {
        b.iter(|| black_box(simple_msg.get_content_text()));
    });

    group.bench_function("extract_complex", |b| {
        b.iter(|| black_box(complex_msg.get_content_text()));
    });

    group.finish();
}

fn benchmark_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");

    // Create multiple files
    let num_files = 10;
    let lines_per_file = 100;
    let mut files = Vec::new();
    let mut _temp_dirs = Vec::new();

    for i in 0..num_files {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join(format!("test{i}.jsonl"));
        let mut file = File::create(&test_file).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {}"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }

        files.push(test_file.to_string_lossy().to_string());
        _temp_dirs.push(temp_dir);
    }

    let _query = parse_query("Message").unwrap();
    let _options = SearchOptions::default();

    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut total = 0;
            for file in &files {
                let content = fs::read_to_string(file).unwrap();
                for line in content.lines() {
                    if line.contains("Message") {
                        total += 1;
                    }
                }
            }
            black_box(total)
        });
    });

    group.bench_function("rayon_parallel", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let total: usize = files
                .par_iter()
                .map(|file| {
                    let content = fs::read_to_string(file).unwrap();
                    content
                        .lines()
                        .filter(|line| line.contains("Message"))
                        .count()
                })
                .sum();
            black_box(total)
        });
    });

    group.finish();
}

fn benchmark_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    let mut temp_dirs = Vec::new();
    let mut test_patterns = Vec::new();

    for size in [100, 1000].iter() {
        let (temp_dir, test_file) = create_test_jsonl(*size, 100);
        let pattern = format!(
            "{}/*.jsonl",
            std::path::Path::new(&test_file).parent().unwrap().display()
        );
        test_patterns.push((*size, pattern.clone()));
        temp_dirs.push(temp_dir);
    }

    for (size, pattern) in test_patterns {
        let query = parse_query("test").unwrap();
        let options = SearchOptions {
            max_results: Some(50),
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::new("search", size), &pattern, |b, pattern| {
            b.iter(|| {
                let engine = SearchEngine::new(options.clone());
                let (results, _, _) = engine.search(pattern, query.clone()).unwrap();
                black_box(results.len())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_file_reading,
    benchmark_json_parsing,
    benchmark_search_matching,
    benchmark_content_extraction,
    benchmark_parallel_processing,
    benchmark_end_to_end
);
criterion_main!(benches);
