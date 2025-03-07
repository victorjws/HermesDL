use indicatif::{MultiProgress, ProgressBar as LibraryProgressBar, ProgressState, ProgressStyle};
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ProgressBar {
    inner: LibraryProgressBar,
    name: String,
}

impl ProgressBar {
    fn new(inner: LibraryProgressBar, name: String) -> Self {
        let pb = Self {
            inner,
            name: name.clone(),
        };
        pb.inner.set_message(name);
        pb
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name.clone();
        self.inner.set_message(name);
    }

    pub fn set_length(&self, length: u64) {
        self.inner.set_length(length);
    }

    pub fn increase(&self, delta: u64) {
        self.inner.inc(delta);
    }

    pub fn finish(&self) {
        self.inner.finish_with_message(format!("{} ✔︎", self.name));
    }

    pub fn finish_and_clear(&self) {
        self.inner.finish_and_clear()
    }
}

pub struct ProgressManager {
    pub main_progress_bar: Arc<RwLock<ProgressBar>>,
    multi_progress: MultiProgress,
    child_style: ProgressStyle,
}

impl ProgressManager {
    pub fn new(name: String) -> Self {
        let multi_progress = MultiProgress::new();
        let style = ProgressStyle::with_template("{msg} {spinner:.green} {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta} / {elapsed_precise})").unwrap().with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()).progress_chars("#>-");

        let main_progress_bar = multi_progress.add(LibraryProgressBar::new(1));
        main_progress_bar.set_style(style.clone());

        let child_style = ProgressStyle::with_template(
            "  [{msg}] {bar:20.green/red} {bytes}/{total_bytes} {pos}/{len}",
        )
        .unwrap()
        .progress_chars("#>-");

        Self {
            main_progress_bar: Arc::new(RwLock::new(ProgressBar::new(main_progress_bar, name))),
            multi_progress,
            child_style,
        }
    }

    pub fn create_new_progress_bar(&self, size: u64, name: String) -> ProgressBar {
        let pb = self.multi_progress.add(LibraryProgressBar::new(size));
        pb.set_style(self.child_style.clone());
        ProgressBar::new(pb, name)
    }
}
