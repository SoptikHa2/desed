#[cfg(target_os = "linux")]
mod inotify;
#[cfg(target_os = "linux")]
pub type FileWatcher = crate::file_watcher::inotify::FileWatcherImpl;
#[cfg(target_os = "linux")]
pub type FileWatch = crate::file_watcher::inotify::FileWatchImpl;

#[cfg(target_os = "freebsd")]
mod kqueue;
#[cfg(target_os = "freebsd")]
pub type FileWatcher = crate::file_watcher::kqueue::FileWatcherImpl;
#[cfg(target_os = "freebsd")]
pub type FileWatch = crate::file_watcher::kqueue::FileWatchImpl;
