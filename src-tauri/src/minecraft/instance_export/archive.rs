use std::fs::{self, File};
use std::io::{copy, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::error::InstanceExportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveSource {
    File(PathBuf),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePlanItem {
    pub zip_path: String,
    pub source: ArchiveSource,
}

pub type ArchivePlan = Vec<ArchivePlanItem>;

/// Write a list of items to a ZIP archive atomically using a temporary file.
pub fn write_archive(
    destination: &Path,
    items: Vec<ArchivePlanItem>,
) -> Result<(), InstanceExportError> {
    let parent = destination.parent().ok_or_else(|| {
        InstanceExportError::io(
            destination,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Missing destination parent directory",
            ),
        )
    })?;

    fs::create_dir_all(parent).map_err(|e| InstanceExportError::io(parent, e))?;

    let tmp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| InstanceExportError::io(parent, e))?;

    let write_res = (|| -> Result<(), InstanceExportError> {
        let mut zip = ZipWriter::new(
            tmp.reopen()
                .map_err(|e| InstanceExportError::io(tmp.path(), e))?,
        );
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(true);

        for item in items {
            let normalized_path = item.zip_path.replace('\\', "/");
            zip.start_file(normalized_path, options)?;

            match item.source {
                ArchiveSource::File(source_path) => {
                    let mut file = File::open(&source_path)
                        .map_err(|e| InstanceExportError::io(&source_path, e))?;
                    copy(&mut file, &mut zip)
                        .map_err(|e| InstanceExportError::io(&source_path, e))?;
                }
                ArchiveSource::Bytes(bytes) => {
                    zip.write_all(&bytes)
                        .map_err(|e| InstanceExportError::Zip(zip::result::ZipError::from(e)))?;
                }
            }
        }

        zip.finish()
            .map_err(InstanceExportError::Zip)?
            .sync_all()
            .map_err(|e| InstanceExportError::io(tmp.path(), e))?;
        Ok(())
    })();

    write_res?;

    tmp.persist(destination)
        .map_err(|e| InstanceExportError::io(destination, e.error))?;

    Ok(())
}
