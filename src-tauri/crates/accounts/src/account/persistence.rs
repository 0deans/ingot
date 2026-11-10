//! The account write boundary permits deterministic failures at each publication step.
use std::{io, io::Write, path::Path};
use tempfile::NamedTempFile;

pub(super) trait Persistence {
    fn stage(&self, directory: &Path) -> io::Result<NamedTempFile>;
    fn write(&self, file: &mut NamedTempFile, bytes: &[u8]) -> io::Result<()>;
    fn flush(&self, file: &NamedTempFile) -> io::Result<()>;
    fn replace(&self, file: NamedTempFile, destination: &Path) -> io::Result<()>;
}

pub(super) struct Filesystem;

impl Persistence for Filesystem {
    fn stage(&self, directory: &Path) -> io::Result<NamedTempFile> {
        NamedTempFile::new_in(directory)
    }

    fn write(&self, file: &mut NamedTempFile, bytes: &[u8]) -> io::Result<()> {
        file.write_all(bytes)
    }

    fn flush(&self, file: &NamedTempFile) -> io::Result<()> {
        file.as_file().sync_all()
    }

    fn replace(&self, file: NamedTempFile, destination: &Path) -> io::Result<()> {
        file.persist(destination)
            .map(|_| ())
            .map_err(|error| error.error)
    }
}
