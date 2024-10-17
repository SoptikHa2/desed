use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::vec::Vec;

pub struct FileWatcherImpl {
    highest_id: usize,
    watches: Vec<FileWatchImpl>,
}

pub struct FileWatchImpl {
    id: usize,
}

impl FileWatcherImpl {
    pub fn init() -> Result<FileWatcherImpl> {
        return Result::Ok(FileWatcherImpl {
            highest_id: 0,
            watches: vec![],
        });
    }

    pub fn add_watch(&mut self, _file_path: &PathBuf) -> Result<&FileWatchImpl> {
        let fw = FileWatchImpl {
            id: self.highest_id,
        };

        self.watches.push(fw);
        self.highest_id += 1;

        return Result::Ok(&self.watches.last().unwrap());
    }

    pub fn rm_watch(&mut self, fw: &FileWatchImpl) -> Result<()> {
        for i in 0..self.watches.len() {
            let item_ref = self.watches.get(i).unwrap();
            if item_ref.id == fw.id {
                self.watches.remove(i);
                return Result::Ok(());
            }
        }

        return Result::Err(Error::new(
            ErrorKind::InvalidInput,
            "Passed FileWatch does not belong to this FileWatcher instance",
        ));
    }

    pub fn start(&mut self) -> Result<()> {
        return Result::Ok(());
    }

    pub fn any_events(&mut self) -> Result<bool> {
        return Result::Ok(false);
    }
}
