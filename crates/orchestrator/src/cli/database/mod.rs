use crate::database::mongodb::MongoDBValidatedArgs;

pub mod mongodb;

#[derive(Debug, Clone)]
pub enum DatabaseParams {
    MongoDB(MongoDBValidatedArgs),
}
