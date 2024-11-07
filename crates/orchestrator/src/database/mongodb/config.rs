#[derive(Debug, Clone)]
pub struct MongoDBParams {
    pub connection_url: String,
    pub database_name: String,
}
