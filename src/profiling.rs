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
            .frequency(1000)
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
