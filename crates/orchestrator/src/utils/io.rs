use std::fs;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn read_file_to_string(file_path: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(contents)
}

pub fn file_exists(file_path: &str) -> bool {
    Path::new(file_path).exists()
}
