
pub mod aws_s3;

#[derive(Clone, Debug)]
pub enum StorageParams {
    AWSS3(aws_s3::AWSS3Params),
}