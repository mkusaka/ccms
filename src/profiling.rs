use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "profiling")]
use pprof::{ProfilerGuard, ProfilerGuardBuilder};
#[cfg(feature = "profiling")]
use std::fs::File;

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "claude_search=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[cfg(feature = "profiling")]
pub struct Profiler {
    guard: Option<ProfilerGuard<'static>>,
}

#[cfg(feature = "profiling")]
impl Profiler {
    pub fn new() -> Result<Self> {
        let guard = ProfilerGuardBuilder::default()
            .frequency(10000) // Increase sampling frequency
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()?;
        Ok(Self { guard: Some(guard) })
    }

    pub fn report(&mut self, path: &str) -> Result<()> {
        if let Some(guard) = self.guard.take() {
            let report = guard.report().build()?;

            // Generate flamegraph
            let file = File::create(format!("{path}.svg"))?;
            report.flamegraph(file)?;

            tracing::info!("Profiling report saved to {}.svg", path);
        }
        Ok(())
    }

    pub fn generate_text_report(&self, report: &pprof::Report) -> Result<String> {
        // Generate human-readable text report
        let mut output = String::new();
        output.push_str("CPU Profiling Report\n");
        output.push_str("===================\n\n");

        // Get top functions by self time - collect function names and counts
        let mut function_counts: std::collections::HashMap<String, isize> =
            std::collections::HashMap::new();

        for (frames, count) in report.data.iter() {
            if let Some(frame) = frames.frames.last() {
                let function_name = frame
                    .first()
                    .map(|s| s.name())
                    .unwrap_or("<unknown>".to_string());
                *function_counts.entry(function_name).or_insert(0) += count;
            }
        }

        let mut functions: Vec<_> = function_counts.into_iter().collect();
        functions.sort_by(|a, b| b.1.abs().cmp(&a.1.abs()));

        output.push_str("Top Functions by Self Time:\n");
        output.push_str("--------------------------\n");

        let total_samples: isize = report.data.values().sum();

        for (i, (function_name, count)) in functions.iter().take(50).enumerate() {
            let percentage = (*count as f64 / total_samples as f64).abs() * 100.0;
            if percentage < 0.1 {
                break;
            }

            output.push_str(&format!(
                "{:3}. {:6.2}% ({:6} samples) {}\n",
                i + 1,
                percentage,
                count,
                function_name
            ));
        }

        Ok(output)
    }

    pub fn report_with_text(&mut self, path: &str) -> Result<String> {
        if let Some(guard) = self.guard.take() {
            let report = guard.report().build()?;

            // Generate all formats
            let svg_file = File::create(format!("{path}.svg"))?;
            report.flamegraph(svg_file)?;

            let text_output = self.generate_text_report(&report)?;
            std::fs::write(format!("{path}.txt"), &text_output)?;

            tracing::info!("Profiling reports saved to {}.svg and {}.txt", path, path);
            Ok(text_output)
        } else {
            Ok("No profiling data available".to_string())
        }
    }
}

#[cfg(not(feature = "profiling"))]
pub struct Profiler;

#[cfg(not(feature = "profiling"))]
impl Profiler {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn report(&mut self, _path: &str) -> Result<()> {
        Ok(())
    }
}
