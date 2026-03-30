//! Port of C++ emFileManSelInfoPanel selection statistics state machine.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanState {
    Costly,
    Wait,
    Scanning,
    Error,
    Success,
}

#[derive(Clone, Debug)]
pub struct ScanDetails {
    pub state: ScanState,
    pub error_message: String,
    pub entries: i32,
    pub hidden_entries: i32,
    pub symbolic_links: i32,
    pub regular_files: i32,
    pub subdirectories: i32,
    pub other_types: i32,
    pub size: u64,
    pub disk_usage: u64,
    pub disk_usage_unknown: bool,
}

impl ScanDetails {
    pub fn new() -> Self {
        Self {
            state: ScanState::Costly,
            error_message: String::new(),
            entries: 0,
            hidden_entries: 0,
            symbolic_links: 0,
            regular_files: 0,
            subdirectories: 0,
            other_types: 0,
            size: 0,
            disk_usage: 0,
            disk_usage_unknown: false,
        }
    }
}

impl Default for ScanDetails {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SelInfoState {
    pub direct: ScanDetails,
    pub recursive: ScanDetails,
}

impl SelInfoState {
    pub fn new() -> Self {
        Self {
            direct: ScanDetails::new(),
            recursive: ScanDetails::new(),
        }
    }
}

impl Default for SelInfoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Process a single entry, updating scan details.
pub fn work_on_detail_entry(
    details: &mut ScanDetails,
    entry: &crate::emDirEntry::emDirEntry,
) {
    details.entries += 1;
    if entry.IsHidden() {
        details.hidden_entries += 1;
    }
    if entry.IsSymbolicLink() {
        details.symbolic_links += 1;
    }
    if entry.IsRegularFile() {
        details.regular_files += 1;
        details.size += entry.GetStat().st_size as u64;
    } else if entry.IsDirectory() {
        details.subdirectories += 1;
    } else {
        details.other_types += 1;
    }
}

/// Process a single entry for recursive scanning (pushes dirs onto stack).
pub fn work_on_detail_entry_with_stack(
    details: &mut ScanDetails,
    entry: &crate::emDirEntry::emDirEntry,
    dir_stack: &mut Vec<String>,
) {
    work_on_detail_entry(details, entry);
    if entry.IsDirectory() {
        dir_stack.push(entry.GetPath().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_costly() {
        let info = SelInfoState::new();
        assert_eq!(info.direct.state, ScanState::Costly);
        assert_eq!(info.recursive.state, ScanState::Costly);
    }

    #[test]
    fn work_on_detail_entry_counts_file() {
        let mut details = ScanDetails::new();
        let e = crate::emDirEntry::emDirEntry::from_path("/dev/null");
        work_on_detail_entry(&mut details, &e);
        assert_eq!(details.entries, 1);
    }

    #[test]
    fn work_on_detail_entry_counts_directory() {
        let mut details = ScanDetails::new();
        let e = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let mut dir_stack = Vec::new();
        work_on_detail_entry_with_stack(&mut details, &e, &mut dir_stack);
        assert_eq!(details.subdirectories, 1);
        assert_eq!(dir_stack.len(), 1);
    }
}
