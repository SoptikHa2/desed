mod inotify;
pub type FileWatcher = crate::file_watcher::inotify::FileWatcherImpl;
pub type FileWatch = crate::file_watcher::inotify::FileWatchImpl;
