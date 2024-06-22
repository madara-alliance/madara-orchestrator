use color_eyre::eyre::Result;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn read_file_to_string(file_path: &PathBuf) -> Result<String> {
    let mut file = File::open(file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(contents)
}
