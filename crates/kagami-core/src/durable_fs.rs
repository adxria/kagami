use std::{fs, io, path::Path};

pub(crate) fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub(crate) fn create_dir_all(path: &Path) -> io::Result<()> {
    let parent = parent_directory(path);
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => return sync_directory(parent),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "Expected a directory.",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound && path.file_name().is_some() => {}
        Err(error) => return Err(error),
    }
    create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    match publish(staging.path(), path) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => {
            sync_directory(parent)
        }
        result => result,
    }
}

#[cfg(not(windows))]
fn publish(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)?;
    sync_directory(parent_directory(destination))
}

#[cfg(not(windows))]
fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory)?.sync_all()
}

pub(crate) fn publish_file(file: tempfile::NamedTempFile, destination: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_NORMAL};

        // Tempfile creates files with FILE_ATTRIBUTE_TEMPORARY on Windows.
        let path = wide(&file.path().canonicalize()?)?;
        if unsafe { SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_NORMAL) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    file.as_file().sync_all()?;
    publish(&file.into_temp_path(), destination)
}

#[cfg(windows)]
fn publish(source: &Path, destination: &Path) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let name = destination.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "A destination name is required.",
        )
    })?;
    let source = source.canonicalize()?;
    let destination = parent_directory(destination).canonicalize()?.join(name);
    let source = wide(&source)?;
    let destination = wide(&destination)?;
    // Both paths are on the same volume. This publishes file and directory
    // renames with write-through before SQLite can commit a replacement.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let mut path: Vec<_> = path.as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Path contains a null character.",
        ));
    }
    path.push(0);
    Ok(path)
}

#[cfg(windows)]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    // New directory entries were published through MoveFileExW.
    Ok(())
}
