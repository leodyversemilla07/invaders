//! Persistent high-score table.
//!
//! The table survives a restart of the game: on the desktop it is kept in a
//! small text file, in the browser it is kept in `localStorage`. All sorting
//! and insertion logic lives in this module, so the platform backends only
//! have to store and retrieve plain numbers.

/// Maximum number of scores kept in the table.
pub const MAX_ENTRIES: usize = 5;

/// One row of the high-score table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Points scored during the finished run.
    pub score: u32,
    /// Wave the player reached during that run.
    pub wave: u32,
}

/// The persistent high-score table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HighScores {
    entries: Vec<Entry>,
    /// Whether the most recently submitted score made the table.
    recorded: bool,
}

impl HighScores {
    /// Reads the table from the platform store.
    pub fn load() -> Self {
        Self::from_entries(backend::load())
    }

    /// Builds a table from raw entries, best score first.
    fn from_entries(entries: Vec<Entry>) -> Self {
        let mut scores = Self {
            entries,
            recorded: false,
        };
        scores.normalize();
        scores
    }

    /// Sorts the table and drops everything beyond [`MAX_ENTRIES`].
    fn normalize(&mut self) {
        self.entries
            .sort_by_key(|entry| std::cmp::Reverse(entry.score));
        self.entries.truncate(MAX_ENTRIES);
    }

    /// The best score so far.
    pub fn best(&self) -> Option<Entry> {
        self.entries.first().copied()
    }

    /// Every entry of the table, best score first.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Whether the latest submitted score made the table.
    pub fn recorded(&self) -> bool {
        self.recorded
    }

    /// Adds a finished run to the table and stores it.
    ///
    /// Returns `true` when the score made it into the table.
    pub fn submit(&mut self, score: u32, wave: u32) -> bool {
        self.recorded = false;

        if !self.insert(score, wave) {
            return false;
        }

        self.recorded = true;
        self.save();
        true
    }

    /// Adds a finished run to the table without storing it.
    ///
    /// Returns `true` when the score made it into the table. A score of zero
    /// is never recorded, and a score that only ties the last place does not
    /// push the older entry out of the table.
    fn insert(&mut self, score: u32, wave: u32) -> bool {
        if score == 0 {
            return false;
        }

        let entry = Entry { score, wave };
        let position = self
            .entries
            .iter()
            .position(|existing| existing.score < entry.score)
            .unwrap_or(self.entries.len());

        if position >= MAX_ENTRIES {
            return false;
        }

        self.entries.insert(position, entry);
        self.entries.truncate(MAX_ENTRIES);
        true
    }

    /// Writes the table to the platform store.
    fn save(&self) {
        backend::save(&self.entries);
    }
}

/// Parses the stored table format: one `score,wave` pair per line.
#[cfg(not(target_arch = "wasm32"))]
fn parse_entries(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| {
            let (score, wave) = line.split_once(',')?;
            let score = score.trim().parse::<u32>().ok()?;
            let wave = wave.trim().parse::<u32>().ok()?;
            Some(Entry { score, wave })
        })
        .filter(|entry| entry.score > 0)
        .collect()
}

/// Renders the table in the stored format.
#[cfg(not(target_arch = "wasm32"))]
fn format_entries(entries: &[Entry]) -> String {
    use std::fmt::Write as _;

    let mut text = String::new();
    for entry in entries {
        let _ = writeln!(text, "{},{}", entry.score, entry.wave);
    }
    text
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::PathBuf;

    use super::{Entry, format_entries, parse_entries};

    pub fn load() -> Vec<Entry> {
        let text = std::fs::read_to_string(storage_path()).unwrap_or_default();
        parse_entries(&text)
    }

    pub fn save(entries: &[Entry]) {
        let path = storage_path();

        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return;
        }

        let _ = std::fs::write(path, format_entries(entries));
    }

    fn storage_path() -> PathBuf {
        data_directory()
            .join("invader-patrol")
            .join("highscores.txt")
    }

    /// The per-user directory where application data is kept.
    fn data_directory() -> PathBuf {
        #[cfg(target_os = "windows")]
        if let Some(directory) = std::env::var_os("APPDATA").filter(|value| !value.is_empty()) {
            return PathBuf::from(directory);
        }

        #[cfg(target_os = "macos")]
        if let Some(home) = home_directory() {
            return home.join("Library").join("Application Support");
        }

        #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
        {
            if let Some(directory) =
                std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty())
            {
                return PathBuf::from(directory);
            }

            if let Some(home) = home_directory() {
                return home.join(".local").join("share");
            }
        }

        home_directory().unwrap_or_else(std::env::temp_dir)
    }

    fn home_directory() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::{Entry, MAX_ENTRIES};

    // The browser keeps the table in `localStorage`; these functions are
    // implemented in `web/index.html`.
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn highscore_count() -> u32;
        fn highscore_score(index: u32) -> u32;
        fn highscore_wave(index: u32) -> u32;
        fn highscore_store(index: u32, score: u32, wave: u32);
        fn highscore_set_length(length: u32);
    }

    pub fn load() -> Vec<Entry> {
        let count = unsafe { highscore_count() };

        (0..count.min(MAX_ENTRIES as u32))
            .map(|index| Entry {
                score: unsafe { highscore_score(index) },
                wave: unsafe { highscore_wave(index) },
            })
            .filter(|entry| entry.score > 0)
            .collect()
    }

    pub fn save(entries: &[Entry]) {
        for (index, entry) in entries.iter().enumerate() {
            unsafe { highscore_store(index as u32, entry.score, entry.wave) };
        }

        unsafe { highscore_set_length(entries.len() as u32) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(score: u32, wave: u32) -> Entry {
        Entry { score, wave }
    }

    #[test]
    fn a_new_table_is_empty() {
        let scores = HighScores::default();
        assert!(scores.entries().is_empty());
        assert_eq!(scores.best(), None);
        assert!(!scores.recorded());
    }

    #[test]
    fn submitting_a_score_fills_the_table() {
        let mut scores = HighScores::default();

        assert!(scores.insert(100, 2));
        assert!(scores.insert(300, 4));
        assert!(scores.insert(200, 3));

        assert_eq!(scores.best(), Some(entry(300, 4)));
        assert_eq!(scores.entries().len(), 3);
    }

    #[test]
    fn the_table_keeps_the_best_scores() {
        let mut scores = HighScores::default();

        for score in [10, 50, 30, 20, 40, 60] {
            scores.insert(score, 1);
        }

        assert_eq!(scores.entries().len(), MAX_ENTRIES);
        assert_eq!(scores.best(), Some(entry(60, 1)));
        assert_eq!(scores.entries()[MAX_ENTRIES - 1], entry(20, 1));
    }

    #[test]
    fn a_score_that_misses_the_table_is_rejected() {
        let mut scores = HighScores::default();

        for score in [500, 400, 300, 200, 100] {
            assert!(scores.insert(score, 1));
        }

        assert!(!scores.insert(50, 1));
        assert_eq!(scores.entries().len(), MAX_ENTRIES);
        assert_eq!(scores.best(), Some(entry(500, 1)));
    }

    #[test]
    fn a_tied_score_does_not_push_the_older_entry_out() {
        let mut scores = HighScores::default();

        for score in [500, 400, 300, 200, 100] {
            scores.insert(score, 1);
        }

        assert!(!scores.insert(100, 9));
        assert_eq!(scores.entries()[MAX_ENTRIES - 1], entry(100, 1));
    }

    #[test]
    fn a_zero_score_is_never_recorded() {
        let mut scores = HighScores::default();
        assert!(!scores.insert(0, 1));
        assert!(scores.entries().is_empty());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn the_table_is_stored_as_score_and_wave_pairs() {
        let entries = vec![entry(120, 3), entry(90, 2)];
        let text = format_entries(&entries);

        assert_eq!(text, "120,3\n90,2\n");
        assert_eq!(parse_entries(&text), entries);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn broken_stored_entries_are_ignored() {
        let text = "120,3\ngarbage\n90\n,2\n0,1\n80,2\n";
        assert_eq!(parse_entries(text), vec![entry(120, 3), entry(80, 2)]);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn the_table_survives_a_reload() {
        // The storage location is derived from an environment variable, so the
        // test can redirect it to a temporary directory. This is the only test
        // that reads or writes the real store.
        let directory =
            std::env::temp_dir().join(format!("invader-patrol-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        // SAFETY: no other test in this binary reads the environment.
        unsafe { std::env::set_var("XDG_DATA_HOME", &directory) };

        let mut scores = HighScores::default();
        assert!(scores.submit(4200, 5));
        assert!(scores.submit(100, 1));

        let reloaded = HighScores::load();
        assert_eq!(reloaded.best(), Some(entry(4200, 5)));
        assert_eq!(reloaded.entries().len(), 2);
        assert_eq!(reloaded.entries()[1], entry(100, 1));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_loaded_table_is_sorted_and_truncated() {
        let scores = HighScores::from_entries(vec![
            entry(10, 1),
            entry(90, 2),
            entry(70, 3),
            entry(80, 4),
            entry(60, 5),
            entry(50, 6),
        ]);

        assert_eq!(scores.entries().len(), MAX_ENTRIES);
        assert_eq!(scores.best(), Some(entry(90, 2)));
        assert_eq!(scores.entries()[1], entry(80, 4));
    }
}
