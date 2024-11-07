use crate::database::mongodb::MongoDBParams;

pub mod mongodb;

#[derive(Debug, Clone)]
pub enum DatabaseParams {
    MongoDB(MongoDBParams),
}
