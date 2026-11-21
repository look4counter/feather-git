use std::{os::windows::ffi::OsStrExt, path::Path};

use windows::{
    Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Storage::FileSystem::{
            FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
            FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FindCloseChangeNotification,
            FindFirstChangeNotificationW, FindNextChangeNotification,
        },
        System::Threading::WaitForSingleObject,
    },
    core::{Error, PCWSTR, Result},
};

pub struct WorktreeWatch(HANDLE);

impl WorktreeWatch {
    pub fn new(root: &Path) -> Result<Self> {
        let path: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        let filter = FILE_NOTIFY_CHANGE_FILE_NAME
            | FILE_NOTIFY_CHANGE_DIR_NAME
            | FILE_NOTIFY_CHANGE_LAST_WRITE
            | FILE_NOTIFY_CHANGE_SIZE;
        // The path remains valid through registration. The returned handle is owned by Self.
        unsafe { FindFirstChangeNotificationW(PCWSTR(path.as_ptr()), true, filter).map(Self) }
    }

    pub fn take_change(&self) -> Result<bool> {
        // Self owns a live notification handle. A zero timeout never blocks the UI thread.
        unsafe {
            match WaitForSingleObject(self.0, 0) {
                WAIT_TIMEOUT => Ok(false),
                WAIT_OBJECT_0 => {
                    FindNextChangeNotification(self.0)?;
                    Ok(true)
                }
                _ => Err(Error::from_thread()),
            }
        }
    }
}

impl Drop for WorktreeWatch {
    fn drop(&mut self) {
        // This handle was created by FindFirstChangeNotificationW.
        let _ = unsafe { FindCloseChangeNotification(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs, thread,
        time::{Duration, Instant},
    };

    fn expect_change(watch: &WorktreeWatch) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !watch.take_change().unwrap() {
            assert!(
                Instant::now() < deadline,
                "directory change was not reported"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn worktree_watch_reports_nested_changes_after_rearming() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let watch = WorktreeWatch::new(dir.path()).unwrap();
        assert!(!watch.take_change().unwrap());

        let file = nested.join("test.txt");
        fs::write(&file, "first").unwrap();
        expect_change(&watch);

        fs::write(&file, "second version").unwrap();
        expect_change(&watch);

        let renamed = nested.join("renamed.txt");
        fs::rename(&file, &renamed).unwrap();
        expect_change(&watch);

        fs::remove_file(renamed).unwrap();
        expect_change(&watch);
    }
}
