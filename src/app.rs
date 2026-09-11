use crate::api::Batch;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    CreatedDesc,
    Status,
}

pub struct App {
    pub batches: Vec<Batch>,
    pub selected: usize,
    pub table_state: ratatui::widgets::TableState,
    pub last_refresh: Option<DateTime<Local>>,
    pub error: Option<String>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub loading: bool,
    pub show_help: bool,
    pub confirm_cancel: bool,
    pub refresh_secs: u64,
    pub seconds_until_refresh: u64,
    pub should_quit: bool,
    pub sort_mode: SortMode,
}

impl App {
    pub fn new(refresh_secs: u64) -> Self {
        let mut table_state = ratatui::widgets::TableState::default();
        table_state.select(Some(0));
        Self {
            batches: Vec::new(),
            selected: 0,
            table_state,
            last_refresh: None,
            error: None,
            status_message: None,
            loading: true,
            show_help: false,
            confirm_cancel: false,
            refresh_secs,
            seconds_until_refresh: refresh_secs,
            should_quit: false,
            sort_mode: SortMode::CreatedDesc,
        }
    }

    pub fn set_batches(&mut self, batches: Vec<Batch>) {
        let mut batches = batches;
        self.sort(&mut batches);
        self.batches = batches;
        self.loading = false;
        self.error = None;
        self.last_refresh = Some(Local::now());
        self.seconds_until_refresh = self.refresh_secs;
        if self.selected >= self.batches.len() && !self.batches.is_empty() {
            self.selected = self.batches.len() - 1;
        }
        self.table_state.select(if self.batches.is_empty() {
            None
        } else {
            Some(self.selected)
        });
    }

    fn sort(&self, batches: &mut [Batch]) {
        match self.sort_mode {
            SortMode::CreatedDesc => batches.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortMode::Status => batches.sort_by(|a, b| {
                a.status
                    .cmp(&b.status)
                    .then(b.created_at.cmp(&a.created_at))
            }),
        }
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::CreatedDesc => SortMode::Status,
            SortMode::Status => SortMode::CreatedDesc,
        };
        let mut batches = std::mem::take(&mut self.batches);
        self.sort(&mut batches);
        self.batches = batches;
    }

    pub fn set_error(&mut self, msg: String) {
        self.loading = false;
        self.error = Some(msg);
    }

    pub fn flash(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn selected_batch(&self) -> Option<&Batch> {
        self.batches.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if self.batches.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.batches.len() - 1);
        self.table_state.select(Some(self.selected));
    }

    pub fn select_prev(&mut self) {
        if self.batches.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.table_state.select(Some(self.selected));
    }

    pub fn select_first(&mut self) {
        if self.batches.is_empty() {
            return;
        }
        self.selected = 0;
        self.table_state.select(Some(self.selected));
    }

    pub fn select_last(&mut self) {
        if self.batches.is_empty() {
            return;
        }
        self.selected = self.batches.len() - 1;
        self.table_state.select(Some(self.selected));
    }

    pub fn is_cancellable(batch: &Batch) -> bool {
        matches!(
            batch.status.as_str(),
            "validating" | "in_progress" | "finalizing"
        )
    }

    pub fn tick_countdown(&mut self) {
        if self.seconds_until_refresh > 0 {
            self.seconds_until_refresh -= 1;
        }
        if let Some((_, at)) = &self.status_message {
            if at.elapsed().as_secs() > 4 {
                self.status_message = None;
            }
        }
    }
}
