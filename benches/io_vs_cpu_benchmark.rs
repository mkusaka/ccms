use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

fn create_test_files(num_files: usize, lines_per_file: usize) -> (tempfile::TempDir, Vec<String>) {
    let temp_dir = tempdir().unwrap();
    let mut files = Vec::new();
    
    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("test{}.txt", i));
        let mut file = File::create(&file_path).unwrap();
        
        // Create larger content for more realistic I/O
        let line = "This is a test line with some content that needs to be processed. ".repeat(10);
        for _ in 0..lines_per_file {
            writeln!(file, "{}", line).unwrap();
        }
        
        files.push(file_path.to_string_lossy().to_string());
    }
    
    (temp_dir, files)
}

fn benchmark_io_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_only");
    let (_temp_dir, files) = create_test_files(50, 1000); // 50 files, 1000 lines each
    
    // Pure I/O - Sequential
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut total_size = 0;
            for file in &files {
                let content = std::fs::read_to_string(file).unwrap();
                total_size += content.len();
            }
            black_box(total_size)
        });
    });
    
    // Pure I/O - Rayon
    group.bench_function("rayon", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let total_size: usize = files
                .par_iter()
                .map(|file| {
                    let content = std::fs::read_to_string(file).unwrap();
                    content.len()
                })
                .sum();
            black_box(total_size)
        });
    });
    
    // Pure I/O - Smol blocking
    group.bench_function("smol_blocking", |b| {
        b.iter(|| {
            let total_size = smol::block_on(async {
                use futures_lite::stream::{self, StreamExt};
                
                let stream = stream::iter(files.iter()).map(|file| {
                    let file = file.clone();
                    smol::spawn(async move {
                        blocking::unblock(move || {
                            let content = std::fs::read_to_string(file).unwrap();
                            content.len()
                        }).await
                    })
                });
                
                let mut total = 0;
                let mut stream = Box::pin(stream);
                while let Some(task) = stream.next().await {
                    total += task.await;
                }
                total
            });
            black_box(total_size)
        });
    });
    
    group.finish();
}

fn benchmark_cpu_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_only");
    
    // Create in-memory data to process
    let data: Vec<String> = (0..50)
        .map(|i| {
            format!("This is test content {} that needs to be processed. ", i).repeat(10000)
        })
        .collect();
    
    // CPU only - Sequential
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut count = 0;
            for content in &data {
                count += content.lines().filter(|line| line.contains("test")).count();
            }
            black_box(count)
        });
    });
    
    // CPU only - Rayon
    group.bench_function("rayon", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let count: usize = data
                .par_iter()
                .map(|content| {
                    content.lines().filter(|line| line.contains("test")).count()
                })
                .sum();
            black_box(count)
        });
    });
    
    // CPU only - Smol
    group.bench_function("smol", |b| {
        b.iter(|| {
            let count = smol::block_on(async {
                use futures_lite::stream::{self, StreamExt};
                
                let stream = stream::iter(data.iter()).map(|content| {
                    let content = content.clone();
                    smol::spawn(async move {
                        content.lines().filter(|line| line.contains("test")).count()
                    })
                });
                
                let mut total = 0;
                let mut stream = Box::pin(stream);
                while let Some(task) = stream.next().await {
                    total += task.await;
                }
                total
            });
            black_box(count)
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_io_only, benchmark_cpu_only);
criterion_main!(benches);