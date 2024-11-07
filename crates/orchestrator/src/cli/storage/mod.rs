use crate::data_storage::aws_s3::config::AWSS3Params;

pub mod aws_s3;

#[derive(Clone, Debug)]
pub enum StorageParams {
    AWSS3(AWSS3Params),
}
