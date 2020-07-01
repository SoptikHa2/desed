use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::vec::Vec;

extern crate inotify;
use inotify::Inotify;

pub struct FileWatcher {
    inotify: Inotify,
    watches: Vec<FileWatch>
}

pub struct FileWatch {
    descriptor: inotify::WatchDescriptor,
}

impl FileWatcher {
    pub fn init() -> Result<FileWatcher>  {
        let ino = match Inotify::init() {
            Ok(i) => i,
            Err(msg) => return Result::Err(msg),
        };

        return Result::Ok(FileWatcher {
            inotify: ino,
            watches: vec![]
        });
    }

    pub fn close(self) -> Result<()> {
        return self.inotify.close();
    }

    pub fn add_watch(&mut self, file_path: &PathBuf) -> Result<&FileWatch> {
        let mask: inotify::WatchMask = inotify::WatchMask::MODIFY;

        let watch = match self.inotify.add_watch(file_path, mask) {
            Ok(w) => w,
            Err(msg) => return Result::Err(msg),
        };

        let fw = FileWatch {
            descriptor: watch,
        };

        self.watches.push(fw);
        return Result::Ok(&self.watches.last().unwrap());
    }

    pub fn rm_watch(&mut self, fw: &FileWatch) -> Result<()> {
        for i in 0..self.watches.len() {
            let item_ref = self.watches.get(i).unwrap();
            if item_ref.descriptor == fw.descriptor {
                let item = self.watches.remove(i);
                return self.inotify.rm_watch(item.descriptor);
            }
        }

        return Result::Err(Error::new(
            ErrorKind::InvalidInput,
            "Passed FileWatch does not belong to this FileWatcher instance"
        ));
    }

    pub fn any_events(&mut self) -> Result<bool> {
        let mut buffer = [0; 1024];
        let events = match self.inotify.read_events(&mut buffer) {
            Result::Ok(ev) => ev,
            Result::Err(err) => return Result::Err(Error::from(err)),
        };

        return Result::Ok(events.count() > 0);
    }
}
