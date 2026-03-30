use std::ffi::CString;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct SharedData {
    path: String,
    name: String,
    target_path: String,
    owner: String,
    group: String,
    hidden: bool,
    stat: libc::stat,
    /// `Some` when the entry is a symlink (holds the lstat result).
    /// `None` when not a symlink — `GetLStat` returns `&self.stat` in that case,
    /// mirroring C++ where `LStat` points to `Stat`.
    lstat: Option<libc::stat>,
    stat_errno: i32,
    lstat_errno: i32,
    target_path_errno: i32,
}

impl Default for SharedData {
    fn default() -> Self {
        Self {
            path: String::new(),
            name: String::new(),
            target_path: String::new(),
            owner: String::new(),
            group: String::new(),
            hidden: false,
            stat: unsafe { std::mem::zeroed() },
            lstat: None,
            stat_errno: 0,
            lstat_errno: 0,
            target_path_errno: 0,
        }
    }
}

impl PartialEq for SharedData {
    fn eq(&self, other: &Self) -> bool {
        self.stat_errno == other.stat_errno
            && self.lstat_errno == other.lstat_errno
            && self.target_path_errno == other.target_path_errno
            && self.path == other.path
            && self.name == other.name
            && self.target_path == other.target_path
            && self.owner == other.owner
            && self.group == other.group
            && self.hidden == other.hidden
            && stat_bytes(&self.stat) == stat_bytes(&other.stat)
            && lstat_bytes(&self.lstat) == lstat_bytes(&other.lstat)
    }
}

fn stat_bytes(s: &libc::stat) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (s as *const libc::stat).cast::<u8>(),
            std::mem::size_of::<libc::stat>(),
        )
    }
}

fn lstat_bytes(s: &Option<libc::stat>) -> Option<&[u8]> {
    s.as_ref().map(stat_bytes)
}

#[derive(Clone, Debug)]
pub struct emDirEntry {
    data: Rc<SharedData>,
}

impl PartialEq for emDirEntry {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data) || *self.data == *other.data
    }
}

impl Eq for emDirEntry {}

impl emDirEntry {
    pub fn new() -> Self {
        Self {
            data: Rc::new(SharedData::default()),
        }
    }

    pub fn from_path(path: &str) -> Self {
        let name = get_name_in_path(path).to_string();
        let mut entry = Self::new();
        entry.priv_load(path.to_string(), name);
        entry
    }

    pub fn from_parent_and_name(parent_path: &str, name: &str) -> Self {
        let path = get_child_path(parent_path, name);
        let mut entry = Self::new();
        entry.priv_load(path, name.to_string());
        entry
    }

    pub fn Load(&mut self, path: &str) {
        let name = get_name_in_path(path).to_string();
        self.priv_load(path.to_string(), name);
    }

    pub fn LoadParentAndName(&mut self, parent_path: &str, name: &str) {
        let path = get_child_path(parent_path, name);
        self.priv_load(path, name.to_string());
    }

    pub fn Clear(&mut self) {
        self.data = Rc::new(SharedData::default());
    }

    pub fn GetPath(&self) -> &str {
        &self.data.path
    }

    pub fn GetName(&self) -> &str {
        &self.data.name
    }

    pub fn GetTargetPath(&self) -> &str {
        &self.data.target_path
    }

    pub fn IsSymbolicLink(&self) -> bool {
        match &self.data.lstat {
            Some(ls) => (ls.st_mode & libc::S_IFMT) == libc::S_IFLNK,
            None => false,
        }
    }

    pub fn IsDirectory(&self) -> bool {
        (self.data.stat.st_mode & libc::S_IFMT) == libc::S_IFDIR
    }

    pub fn IsRegularFile(&self) -> bool {
        (self.data.stat.st_mode & libc::S_IFMT) == libc::S_IFREG
    }

    pub fn IsHidden(&self) -> bool {
        self.data.hidden
    }

    pub fn GetStat(&self) -> &libc::stat {
        &self.data.stat
    }

    /// Returns the lstat result. If not a symlink, returns `&self.GetStat()`
    /// (mirroring C++ where `LStat` points to `Stat`).
    pub fn GetLStat(&self) -> &libc::stat {
        match &self.data.lstat {
            Some(ls) => ls,
            None => &self.data.stat,
        }
    }

    pub fn GetOwner(&self) -> &str {
        &self.data.owner
    }

    pub fn GetGroup(&self) -> &str {
        &self.data.group
    }

    pub fn GetTargetPathErrNo(&self) -> i32 {
        self.data.target_path_errno
    }

    pub fn GetStatErrNo(&self) -> i32 {
        self.data.stat_errno
    }

    pub fn GetLStatErrNo(&self) -> i32 {
        self.data.lstat_errno
    }

    pub fn GetDataRefCount(&self) -> usize {
        Rc::strong_count(&self.data)
    }

    fn priv_load(&mut self, path: String, name: String) {
        let mut d = SharedData::default();
        d.path = path;
        d.name = name;
        d.target_path = d.path.clone();

        let c_path = match CString::new(d.path.as_bytes()) {
            Ok(p) => p,
            Err(_) => {
                d.lstat_errno = libc::EINVAL;
                d.stat_errno = libc::EINVAL;
                self.data = Rc::new(d);
                return;
            }
        };

        let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
        let lstat_ret = unsafe { libc::lstat(c_path.as_ptr(), &mut stat_buf) };

        if lstat_ret != 0 {
            // lstat failed
            d.lstat_errno = errno();
            // Try stat as fallback
            let stat_ret = unsafe { libc::stat(c_path.as_ptr(), &mut stat_buf) };
            if stat_ret != 0 {
                d.stat_errno = errno();
                // stat_buf stays zeroed
            } else {
                // stat succeeded but lstat failed — store a zeroed lstat
                d.stat = stat_buf;
                d.lstat = Some(unsafe { std::mem::zeroed() });
            }
        } else if (stat_buf.st_mode & libc::S_IFMT) == libc::S_IFLNK {
            // lstat succeeded, it's a symlink
            d.lstat = Some(stat_buf);
            // Now stat the target
            let stat_ret = unsafe { libc::stat(c_path.as_ptr(), &mut stat_buf) };
            if stat_ret != 0 {
                d.stat_errno = errno();
                d.stat = unsafe { std::mem::zeroed() };
            } else {
                d.stat = stat_buf;
            }
            // readlink for target path
            let mut buf = vec![0u8; (libc::PATH_MAX as usize) + 1];
            let len =
                unsafe { libc::readlink(c_path.as_ptr(), buf.as_mut_ptr().cast(), buf.len() - 1) };
            if len < 0 {
                d.target_path_errno = errno();
                d.target_path = String::new();
            } else {
                buf.truncate(len as usize);
                d.target_path = String::from_utf8_lossy(&buf).into_owned();
            }
        } else {
            // lstat succeeded, not a symlink
            d.stat = stat_buf;
            // lstat stays None — GetLStat returns &stat
        }

        // Owner name via getpwuid_r
        d.owner = get_owner_name(d.stat.st_uid);

        // Group name via getgrgid_r
        d.group = get_group_name(d.stat.st_gid);

        // Hidden = name starts with '.'
        d.hidden = d.name.starts_with('.');

        self.data = Rc::new(d);
    }
}

impl Default for emDirEntry {
    fn default() -> Self {
        Self::new()
    }
}

fn get_name_in_path(path: &str) -> &str {
    match path.rfind('/') {
        Some(pos) => &path[pos + 1..],
        None => path,
    }
}

fn get_child_path(parent: &str, name: &str) -> String {
    if parent.ends_with('/') {
        format!("{parent}{name}")
    } else {
        format!("{parent}/{name}")
    }
}

fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

fn get_owner_name(uid: libc::uid_t) -> String {
    let mut buf = vec![0u8; 4096];
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    let ret = unsafe {
        libc::getpwuid_r(
            uid,
            &mut pwd,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut result,
        )
    };
    if ret == 0 && !result.is_null() {
        let name = unsafe { std::ffi::CStr::from_ptr(pwd.pw_name) };
        name.to_string_lossy().into_owned()
    } else {
        format!("{uid}")
    }
}

fn get_group_name(gid: libc::gid_t) -> String {
    let mut buf = vec![0u8; 4096];
    let mut grp: libc::group = unsafe { std::mem::zeroed() };
    let mut result: *mut libc::group = std::ptr::null_mut();
    let ret = unsafe {
        libc::getgrgid_r(
            gid,
            &mut grp,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut result,
        )
    };
    if ret == 0 && !result.is_null() {
        let name = unsafe { std::ffi::CStr::from_ptr(grp.gr_name) };
        name.to_string_lossy().into_owned()
    } else {
        format!("{gid}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_entry_has_empty_fields() {
        let e = emDirEntry::new();
        assert!(e.GetPath().is_empty());
        assert!(e.GetName().is_empty());
        assert!(e.GetTargetPath().is_empty());
        assert!(e.GetOwner().is_empty());
        assert!(e.GetGroup().is_empty());
        assert!(!e.IsHidden());
        assert!(!e.IsSymbolicLink());
        assert!(!e.IsDirectory());
        assert!(!e.IsRegularFile());
        assert_eq!(e.GetStatErrNo(), 0);
        assert_eq!(e.GetLStatErrNo(), 0);
        assert_eq!(e.GetTargetPathErrNo(), 0);
    }

    #[test]
    fn cow_clone_shares_data() {
        let e1 = emDirEntry::new();
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn load_real_file() {
        let e = emDirEntry::from_path("/dev/null");
        assert_eq!(e.GetPath(), "/dev/null");
        assert_eq!(e.GetName(), "null");
        assert!(e.IsRegularFile() || !e.IsDirectory()); // /dev/null is a char device
        assert_eq!(e.GetStatErrNo(), 0);
        assert!(!e.GetOwner().is_empty());
        assert!(!e.GetGroup().is_empty());
    }

    #[test]
    fn load_parent_and_name() {
        let e = emDirEntry::from_parent_and_name("/dev", "null");
        assert_eq!(e.GetPath(), "/dev/null");
        assert_eq!(e.GetName(), "null");
    }

    #[test]
    fn load_directory() {
        let e = emDirEntry::from_path("/tmp");
        assert!(e.IsDirectory());
        assert!(!e.IsRegularFile());
    }

    #[test]
    fn hidden_file_detection() {
        let dir = std::env::temp_dir();
        let hidden_path = dir.join(".test_hidden_emfileman");
        std::fs::write(&hidden_path, "test").unwrap();
        let e = emDirEntry::from_path(hidden_path.to_str().unwrap());
        assert!(e.IsHidden());
        std::fs::remove_file(&hidden_path).unwrap();
    }

    #[test]
    fn symlink_detection() {
        let dir = std::env::temp_dir();
        let target = dir.join("emfileman_symlink_target");
        let link = dir.join("emfileman_symlink_link");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_file(&target);
        std::fs::write(&target, "data").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let e = emDirEntry::from_path(link.to_str().unwrap());
        assert!(e.IsSymbolicLink());
        assert!(e.IsRegularFile()); // follows symlink for stat
        assert_eq!(e.GetTargetPathErrNo(), 0);
        assert!(!e.GetTargetPath().is_empty());

        std::fs::remove_file(&link).unwrap();
        std::fs::remove_file(&target).unwrap();
    }

    #[test]
    fn nonexistent_path() {
        let e = emDirEntry::from_path("/nonexistent_emfileman_test_path");
        assert_ne!(e.GetStatErrNo(), 0);
        assert_ne!(e.GetLStatErrNo(), 0);
    }

    #[test]
    fn equality() {
        let e1 = emDirEntry::from_path("/dev/null");
        let e2 = emDirEntry::from_path("/dev/null");
        assert_eq!(e1, e2);

        let e3 = emDirEntry::from_path("/tmp");
        assert_ne!(e1, e3);
    }
}
