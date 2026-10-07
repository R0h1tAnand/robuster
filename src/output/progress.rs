//! Progress bar and status display

use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Create a styled progress bar
pub fn create_progress_bar(total: u64, quiet: bool) -> Option<ProgressBar> {
    if quiet {
        return None;
    }

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}"
        )
        .unwrap()
        .progress_chars("█▓▒░ ")
    );

    Some(pb)
}

/// Create a spinner for indeterminate progress
#[allow(dead_code)]
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Progress tracker with atomic counters
#[derive(Clone)]
pub struct ProgressTracker {
    pub bar: Option<Arc<ProgressBar>>,
    found: Arc<std::sync::atomic::AtomicUsize>,
    errors: Arc<std::sync::atomic::AtomicUsize>,
}

impl ProgressTracker {
    pub fn new(total: u64, quiet: bool) -> Self {
        let bar = create_progress_bar(total, quiet).map(Arc::new);
        Self {
            bar,
            found: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            errors: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn inc(&self) {
        if let Some(ref bar) = self.bar {
            bar.inc(1);
        }
    }

    pub fn inc_found(&self) {
        self.found
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.update_message();
    }

    pub fn inc_error(&self) {
        self.errors
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Print a line above the progress bar without disrupting its position.
    /// Uses `bar.println()` so indicatif handles cursor management correctly.
    pub fn println(&self, line: &str) {
        if let Some(ref bar) = self.bar {
            bar.println(line);
        } else {
            println!("{}", line);
        }
    }

    pub fn finish(&self) {
        if let Some(ref bar) = self.bar {
            bar.finish_with_message("done");
        }
    }

    #[allow(dead_code)]
    pub fn found_count(&self) -> usize {
        self.found.load(std::sync::atomic::Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn error_count(&self) -> usize {
        self.errors.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn update_message(&self) {
        if let Some(ref bar) = self.bar {
            let found = self.found.load(std::sync::atomic::Ordering::Relaxed);
            bar.set_message(format!("Found: {}", found));
        }
    }
}
