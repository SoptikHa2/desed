use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::vec::Vec;

extern crate inotify;
use inotify::Inotify;

pub struct FileWatcherImpl {
    inotify: Inotify,
    watches: Vec<FileWatchImpl>,
}

pub struct FileWatchImpl {
    descriptor: inotify::WatchDescriptor,
}

impl FileWatcherImpl {
    pub fn init() -> Result<FileWatcherImpl> {
        let ino = Inotify::init()?;

        Result::Ok(FileWatcherImpl {
            inotify: ino,
            watches: vec![],
        })
    }

    pub fn add_watch(&mut self, file_path: &PathBuf) -> Result<&FileWatchImpl> {
        let mask: inotify::WatchMask = inotify::WatchMask::MODIFY;

        let watch = self.inotify.watches().add(file_path, mask)?;
        let fw = FileWatchImpl { descriptor: watch };

        self.watches.push(fw);
        Result::Ok(self.watches.last().unwrap())
    }

    pub fn rm_watch(&mut self, fw: &FileWatchImpl) -> Result<()> {
        for i in 0..self.watches.len() {
            let item_ref = self.watches.get(i).unwrap();
            if item_ref.descriptor == fw.descriptor {
                let item = self.watches.remove(i);
                return self.inotify.watches().remove(item.descriptor);
            }
        }

        Result::Err(Error::new(
            ErrorKind::InvalidInput,
            "Passed FileWatch does not belong to this FileWatcher instance",
        ))
    }

    pub fn start(&mut self) -> Result<()> {
        Result::Ok(())
    }

    pub fn any_events(&mut self) -> Result<bool> {
        let mut buffer = [0; 1024];
        let events = self.inotify.read_events(&mut buffer)?;

        Result::Ok(events.count() > 0)
    }
}
