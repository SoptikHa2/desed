extern crate cfg_if;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod inotify;
        pub type FileWatcher = crate::file_watcher::inotify::FileWatcherImpl;
        pub type FileWatch = crate::file_watcher::inotify::FileWatchImpl;
    } else if #[cfg(any(target_os="darwin", target_os="dragonfly", target_os="freebsd", target_os="netbsd", target_os="openbsd"))] {
        mod kqueue;
        pub type FileWatcher = crate::file_watcher::kqueue::FileWatcherImpl;
        pub type FileWatch = crate::file_watcher::kqueue::FileWatchImpl;
    } else {
        mod mock;
        pub type FileWatcher = crate::file_watcher::mock::FileWatcherImpl;
        pub type FileWatch = crate::file_watcher::mock::FileWatchImpl;
    }
}
