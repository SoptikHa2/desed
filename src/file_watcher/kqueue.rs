use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::vec::Vec;

extern crate kqueue;
use kqueue::Watcher as KQueue;
use kqueue::FilterFlag as FilterFlag;

const FILTER: kqueue::EventFilter = kqueue::EventFilter::EVFILT_VNODE;

pub struct FileWatcherImpl {
    kq: KQueue,
    watches: Vec<FileWatchImpl>
}

pub struct FileWatchImpl {
    file: File,
}

impl FileWatcherImpl {
    pub fn init() -> Result<FileWatcherImpl> {
        let kq = match KQueue::new() {
            Ok(value) => value,
            Err(msg) => return Result::Err(msg),
        };

        return Result::Ok(FileWatcherImpl {
            kq,
            watches: vec![]
        });
    }

    pub fn add_watch(&mut self, file_path: &PathBuf) -> Result<&FileWatchImpl> {
        let flags: FilterFlag =
            FilterFlag::NOTE_WRITE |
            FilterFlag::NOTE_EXTEND |
            FilterFlag::NOTE_RENAME |
            FilterFlag::NOTE_DELETE |
            FilterFlag::NOTE_LINK;

        let file = File::open(file_path)?;
        let _ = match self.kq.add_file(&file, FILTER, flags) {
            Ok(w) => w,
            Err(msg) => return Result::Err(msg),
        };

        let fw = FileWatchImpl {
            file
        };

        self.watches.push(fw);
        return Result::Ok(&self.watches.last().unwrap());
    }

    pub fn rm_watch(&mut self, fw: &FileWatchImpl) -> Result<()> {
        for i in 0..self.watches.len() {
            let item_ref = self.watches.get(i).unwrap();
            if item_ref as *const FileWatchImpl == fw as *const FileWatchImpl {
                let item = self.watches.remove(i);
                return self.kq.remove_file(&item.file, FILTER);
            }
        }

        return Result::Err(Error::new(
            ErrorKind::InvalidInput,
            "Passed FileWatch does not belong to this FileWatcher instance"
        ));
    }

    pub fn start(&mut self) -> Result<()> {
        return self.kq.watch();
    }

    pub fn any_events(&mut self) -> Result<bool> {
        match self.kq.poll(None) {
            Some(_) => return Result::Ok(true),
            None => return Result::Ok(false),
        }
    }
}
