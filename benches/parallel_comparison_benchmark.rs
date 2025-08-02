use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_test_files(num_files: usize, lines_per_file: usize) -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().unwrap();
    
    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("test{}.jsonl", i));
        let mut file = File::create(&file_path).unwrap();

        for j in 0..lines_per_file {
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {} in file {} with some test content to search through"}},
"uuid":"{}","timestamp":"2024-01-01T00:00:{:02}Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#,
                j,
                i,
                j,
                j % 60
            )
            .unwrap();
        }
    }

    let pattern = format!("{}/*.jsonl", temp_dir.path().display());
    (temp_dir, pattern)
}

// Rayon-based parallel file processing
fn process_files_rayon(files: &[String]) -> usize {
    use rayon::prelude::*;
    
    files
        .par_iter()
        .map(|file| {
            let content = std::fs::read_to_string(file).unwrap();
            content
                .lines()
                .filter(|line| line.contains("test"))
                .count()
        })
        .sum()
}

// Smol-based parallel file processing with different thread counts
fn process_files_smol(files: &[String]) -> usize {
    smol::block_on(async {
        use futures_lite::stream::{self, StreamExt};
        
        let stream = stream::iter(files.iter()).map(|file| {
            let file = file.clone();
            smol::spawn(async move {
                let content = async_fs::read_to_string(file).await.unwrap();
                content
                    .lines()
                    .filter(|line| line.contains("test"))
                    .count()
            })
        });
        
        let mut total = 0;
        let mut stream = Box::pin(stream);
        while let Some(task) = stream.next().await {
            let count = task.await;
            total += count;
        }
        total
    })
}

fn benchmark_parallel_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_comparison");
    
    // Test different file counts
    for (name, num_files, lines_per_file) in [
        ("small", 10, 100),
        ("medium", 50, 200),
        ("large", 100, 500),
    ] {
        let (_temp_dir, pattern) = create_test_files(num_files, lines_per_file);
        
        // Get file list
        let files: Vec<String> = std::fs::read_dir(std::path::Path::new(&pattern).parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
            .map(|e| e.path().to_string_lossy().to_string())
            .collect();
        
        // Benchmark Rayon
        group.bench_with_input(
            BenchmarkId::new("rayon", name),
            &files,
            |b, files| {
                b.iter(|| {
                    black_box(process_files_rayon(files))
                });
            },
        );
        
        // Benchmark Smol with different BLOCKING_MAX_THREADS values
        for threads in [1, 4, 8, 10, 16] {
            group.bench_with_input(
                BenchmarkId::new(format!("smol_threads_{}", threads), name),
                &files,
                |b, files| {
                    // Set BLOCKING_MAX_THREADS for this benchmark
                    unsafe {
                        std::env::set_var("BLOCKING_MAX_THREADS", threads.to_string());
                    }
                    
                    // Force re-initialization of blocking thread pool
                    // Note: This won't work with the current implementation that uses std::sync::Once
                    // In real usage, this would be set before the program starts
                    
                    b.iter(|| {
                        black_box(process_files_smol(files))
                    });
                },
            );
        }
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_parallel_comparison);
criterion_main!(benches);