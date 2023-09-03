use crate::error::Error;
use std::path::Path;

#[macro_export]
macro_rules! EchoPathBufToString {
    ($x:expr) => {{
        $x.to_str().unwrap().to_string()
    }};
}

#[macro_export]
macro_rules! EchoStringToPathBuf {
    ($x:expr) => {{
        std::path::PathBuf::from($x)
    }};
}

pub fn create_dir_sync<P: AsRef<Path>>(path: &P) -> Result<(), Error> {
    use std::fs;

    if let Ok(attr) = fs::metadata(&path) {
        if attr.is_dir() {
            // #[cfg(unix)]
            // {
            //     let mut perms
            //         = fs::metadata(&path).await?.permissions();

            //         perms.set_mode(0o644);
            //         fs::set_permissions(&path, perms).await?;
            // }

            return Ok(());
        }

        return Err(Error::InvalidPath(format!(
            "path is not directory,{}",
            path.as_ref().display()
        )));
    }

    fs::create_dir_all(&path).map_err(|e| Error::IoError(e.to_string()))?;

    Ok(())
}

// async
pub async fn create_dir<P: AsRef<Path>>(path: &P) -> Result<(), Error> {
    use tokio::fs;

    if let Ok(attr) = fs::metadata(&path).await {
        if attr.is_dir() {
            // #[cfg(unix)]
            // {
            //     let mut perms
            //         = fs::metadata(&path).await?.permissions();

            //         perms.set_mode(0o644);
            //         fs::set_permissions(&path, perms).await?;
            // }

            return Ok(());
        }

        return Err(Error::InvalidPath(format!(
            "path is not directory,{}",
            path.as_ref().display()
        )));
    }

    fs::create_dir_all(&path)
        .await
        .map_err(|e| Error::IoError(e.to_string()))?;

    Ok(())
}

pub async fn cleanup_dir<P: AsRef<Path>>(path: &P) -> Result<(), Error> {
    use tokio::fs;

    if let Ok(attr) = fs::metadata(&path).await {
        if attr.is_dir() {
            for entry in
                std::fs::read_dir(path.as_ref()).map_err(|e| Error::IoError(e.to_string()))?
            {
                let child_path = entry.map_err(|e| Error::IoError(e.to_string()))?.path();

                if child_path.is_dir() {
                    fs::remove_dir_all(&child_path)
                        .await
                        .map_err(|e| Error::IoError(e.to_string()))?;
                } else {
                    fs::remove_file(&child_path)
                        .await
                        .map_err(|e| Error::IoError(e.to_string()))?;
                }
            }
        } else {
            return Err(Error::InvalidPath(format!(
                "path is not directory,{}",
                path.as_ref().display()
            )));
        }
    }

    Err(Error::InvalidPath(format!(
        "failed to access path metadata,{}",
        path.as_ref().display()
    )))
}
