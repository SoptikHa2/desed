use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::vec::Vec;

extern crate kqueue;
use kqueue::FilterFlag;
use kqueue::Watcher as KQueue;

const FILTER: kqueue::EventFilter = kqueue::EventFilter::EVFILT_VNODE;

pub struct FileWatcherImpl {
    kq: KQueue,
    watches: Vec<FileWatchImpl>,
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

        Ok(FileWatcherImpl {
            kq,
            watches: vec![],
        })
    }

    pub fn add_watch(&mut self, file_path: &PathBuf) -> Result<&FileWatchImpl> {
        let flags: FilterFlag = FilterFlag::NOTE_WRITE
            | FilterFlag::NOTE_EXTEND
            | FilterFlag::NOTE_RENAME
            | FilterFlag::NOTE_DELETE
            | FilterFlag::NOTE_LINK;

        let file = File::open(file_path)?;
        self.kq.add_file(&file, FILTER, flags)?;

        let fw = FileWatchImpl { file };

        self.watches.push(fw);
        Ok(self.watches.last().unwrap())
    }

    pub fn rm_watch(&mut self, fw: &FileWatchImpl) -> Result<()> {
        for i in 0..self.watches.len() {
            let item_ref = self.watches.get(i).unwrap();
            if std::ptr::eq(item_ref, fw) {
                let item = self.watches.remove(i);
                return self.kq.remove_file(&item.file, FILTER);
            }
        }

        Err(Error::new(
            ErrorKind::InvalidInput,
            "Passed FileWatch does not belong to this FileWatcher instance",
        ))
    }

    pub fn start(&mut self) -> Result<()> {
        self.kq.watch()
    }

    pub fn any_events(&mut self) -> Result<bool> {
        match self.kq.poll(None) {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }
}
