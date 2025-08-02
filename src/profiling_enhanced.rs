use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::info;
#[cfg(feature = "profiling")]
use pprof::{ProfilerGuard, ProfilerGuardBuilder};

/// Enhanced profiling system with multiple strategies
pub struct EnhancedProfiler {
    #[cfg(feature = "profiling")]
    pprof_guard: Option<ProfilerGuard<'static>>,
    start_time: Instant,
    profile_name: String,
}

impl EnhancedProfiler {
    pub fn new(profile_name: &str) -> Result<Self> {
        let start_time = Instant::now();

        #[cfg(feature = "profiling")]
        {
            // Initialize pprof with better settings
            let pprof_guard = ProfilerGuardBuilder::default()
                .frequency(1000) // 1kHz sampling
                .blocklist(&["libc", "libgcc", "pthread", "vdso", "__pthread"]) // Exclude system libs
                .build()?;

            info!("Enhanced profiling started for: {}", profile_name);

            Ok(Self {
                pprof_guard: Some(pprof_guard),
                start_time,
                profile_name: profile_name.to_string(),
            })
        }

        #[cfg(not(feature = "profiling"))]
        {
            Ok(Self {
                start_time,
                profile_name: profile_name.to_string(),
            })
        }
    }

    #[cfg(feature = "profiling")]
    pub fn generate_comprehensive_report(&mut self, output_path: &str) -> Result<String> {
        let elapsed = self.start_time.elapsed();
        let mut report = String::new();

        report.push_str(&format!(
            "=== Enhanced Profiling Report: {} ===\n",
            self.profile_name
        ));
        report.push_str(&format!("Total runtime: {:.3}s\n\n", elapsed.as_secs_f64()));

        // Generate pprof reports
        if let Some(guard) = self.pprof_guard.take() {
            let pprof_report = guard.report().build()?;

            // Save flamegraph
            let svg_file = std::fs::File::create(format!("{output_path}.svg"))?;
            pprof_report.flamegraph(svg_file)?;

            // Generate text report
            let text_report = self.generate_pprof_text_report(&pprof_report)?;
            report.push_str(&text_report);

            // Skip protobuf for now - pprof API has changed

            info!("Profiling reports saved to {}.{{svg,txt,pb}}", output_path);
        }

        // Save the comprehensive report
        std::fs::write(format!("{output_path}_comprehensive.txt"), &report)?;

        Ok(report)
    }

    #[cfg(feature = "profiling")]
    fn generate_pprof_text_report(&self, report: &pprof::Report) -> Result<String> {
        let mut output = String::new();
        output.push_str("CPU Profiling Report (pprof)\n");
        output.push_str("============================\n\n");

        // Collect function statistics
        let mut function_stats: std::collections::HashMap<String, (isize, Duration)> =
            std::collections::HashMap::new();

        let total_samples: isize = report.data.values().sum();
        let sample_period = Duration::from_micros(1000); // 1ms per sample at 1kHz

        for (frames, count) in report.data.iter() {
            // Look at all frames in the stack, not just the leaf
            for (depth, frame_id) in frames.frames.iter().enumerate() {
                if let Some(frame) = frame_id.first() {
                    let function_name = frame.name();

                    // Filter out system functions and focus on app code
                    if !function_name.contains("pthread")
                        && !function_name.contains("__libc")
                        && !function_name.contains("syscall")
                        && !function_name.contains("<unknown>")
                    {
                        let entry = function_stats
                            .entry(function_name.clone())
                            .or_insert((0, Duration::ZERO));

                        // Weight by depth - leaf functions get full count, parents get partial
                        let weight = if depth == 0 { 1.0 } else { 0.5 };
                        entry.0 += (*count as f64 * weight) as isize;
                        entry.1 += sample_period * (*count as u32) * weight as u32;
                    }
                }
            }
        }

        // Sort by sample count
        let mut sorted_functions: Vec<_> = function_stats.into_iter().collect();
        sorted_functions.sort_by(|a, b| b.1.0.abs().cmp(&a.1.0.abs()));

        output.push_str("Top Functions by CPU Time:\n");
        output.push_str("--------------------------\n");

        for (i, (function_name, (count, est_time))) in sorted_functions.iter().take(50).enumerate()
        {
            let percentage = (*count as f64 / total_samples as f64).abs() * 100.0;
            if percentage < 0.1 {
                break;
            }

            // Clean up function names for readability
            let clean_name = self.clean_function_name(function_name);

            output.push_str(&format!(
                "{:3}. {:6.2}% ({:6} samples, ~{:>6.1}ms) {}\n",
                i + 1,
                percentage,
                count,
                est_time.as_secs_f64() * 1000.0,
                clean_name
            ));
        }

        output.push_str(&format!("\nTotal samples: {total_samples}\n"));
        output.push_str("Sampling frequency: 1000 Hz\n");
        output.push_str(&format!(
            "Estimated CPU time: {:.3}s\n",
            (total_samples as f64 * sample_period.as_secs_f64())
        ));

        Ok(output)
    }

    #[cfg(feature = "profiling")]
    fn clean_function_name(&self, name: &str) -> String {
        // Remove common Rust mangling patterns
        let clean = name
            .replace("::h", "::") // Remove hash suffixes
            .split("::h")
            .next()
            .unwrap_or(name) // Cut at hash
            .replace("_{{closure}}", "[closure]")
            .replace("_$u7b$$u7b$", "{{")
            .replace("$u7d$$u7d$", "}}")
            .replace("$LT$", "<")
            .replace("$GT$", ">")
            .replace("$C$", ",")
            .replace("$u20$", " ");

        // Shorten common prefixes
        let short = clean
            .replace("ccms::search::", "search::")
            .replace("tokio::runtime::", "tokio::")
            .replace("futures::", "fut::")
            .replace("std::sync::", "sync::")
            .replace("core::", "");

        // Truncate very long names
        if short.len() > 100 {
            format!("{}...", &short[..97])
        } else {
            short
        }
    }

    #[cfg(not(feature = "profiling"))]
    pub fn generate_comprehensive_report(&mut self, _output_path: &str) -> Result<String> {
        Ok("Profiling not enabled. Build with --features profiling".to_string())
    }
}


#[derive(Debug, Clone)]
pub struct OperationProfile {
    pub name: String,
    pub total_time: Duration,
    pub poll_count: usize,
    pub poll_times: Vec<Duration>,
}

impl OperationProfile {
    pub fn avg_poll_time(&self) -> Duration {
        if self.poll_times.is_empty() {
            Duration::ZERO
        } else {
            let total: Duration = self.poll_times.iter().sum();
            total / self.poll_times.len() as u32
        }
    }

    pub fn max_poll_time(&self) -> Duration {
        self.poll_times
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}


/// Helper macro to profile code blocks
#[macro_export]
macro_rules! profile_block {
    ($name:expr, $code:block) => {{
        let _start = std::time::Instant::now();
        let _result = $code;
        let _elapsed = _start.elapsed();
        tracing::debug!(
            block = $name,
            elapsed_ms = _elapsed.as_millis(),
            "Block execution time"
        );
        _result
    }};
}
