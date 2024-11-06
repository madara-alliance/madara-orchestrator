pub mod mongodb;

#[derive(Debug, Clone)]
pub enum DatabaseParams {
    MongoDB(mongodb::MongoDBParams),
}