use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressTracker {
    bar: ProgressBar,
}

impl ProgressTracker {
    pub fn new(total: u64, description: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"));
        bar.set_message(description.to_string());

        ProgressTracker {
            bar,
        }
    }

    pub fn increment(&self, amount: u64) {
        self.bar.inc(amount);
    }

    pub fn finish(&self) {
        self.bar.finish_with_message("Completed");
    }

    pub fn set_message(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }
}