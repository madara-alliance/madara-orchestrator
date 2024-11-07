use crate::database::mongodb::config::MongoDBParams;

pub mod mongodb;

#[derive(Debug, Clone)]
pub enum DatabaseParams {
    MongoDB(MongoDBParams),
}
