use ccms::discover_claude_files;
use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

fn setup_test_project_structure(
    num_projects: usize,
    files_per_project: usize,
) -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let projects_dir = temp_dir.path().join(".claude").join("projects");
    fs::create_dir_all(&projects_dir).unwrap();

    for i in 0..num_projects {
        let project_dir = projects_dir.join(format!("project-{i}"));
        fs::create_dir_all(&project_dir).unwrap();

        for j in 0..files_per_project {
            let file_path = project_dir.join(format!("session-{j}.jsonl"));
            let mut file = File::create(&file_path).unwrap();
            writeln!(
                file,
                r#"{{"type":"user","message":{{"role":"user","content":"Test message {j} in project {i}"}},
"uuid":"{j}","timestamp":"2024-01-01T00:00:00Z","sessionId":"session1",
"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","version":"1.0"}}"#
            )
            .unwrap();
        }

        // Add some non-jsonl files to make it more realistic
        for k in 0..3 {
            let file_path = project_dir.join(format!("other-{k}.txt"));
            File::create(&file_path)
                .unwrap()
                .write_all(b"other file")
                .unwrap();
        }
    }

    temp_dir
}

fn benchmark_file_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_discovery");

    // Small scale: 10 projects, 5 files each
    let small_temp = setup_test_project_structure(10, 5);
    let small_pattern = format!("{}/**/*.jsonl", small_temp.path().display());

    group.bench_function("small_10x5", |b| {
        b.iter(|| {
            let files = discover_claude_files(Some(&small_pattern)).unwrap();
            black_box(files.len())
        });
    });

    // Medium scale: 50 projects, 10 files each
    let medium_temp = setup_test_project_structure(50, 10);
    let medium_pattern = format!("{}/**/*.jsonl", medium_temp.path().display());

    group.bench_function("medium_50x10", |b| {
        b.iter(|| {
            let files = discover_claude_files(Some(&medium_pattern)).unwrap();
            black_box(files.len())
        });
    });

    // Large scale: 100 projects, 20 files each
    let large_temp = setup_test_project_structure(100, 20);
    let large_pattern = format!("{}/**/*.jsonl", large_temp.path().display());

    group.bench_function("large_100x20", |b| {
        b.iter(|| {
            let files = discover_claude_files(Some(&large_pattern)).unwrap();
            black_box(files.len())
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_file_discovery);
criterion_main!(benches);
