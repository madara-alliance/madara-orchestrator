use clap::Args;

/// Parameters used to config MongoDB.
#[derive(Debug, Clone, Args)]
pub struct MongoDBParams {
    /// The connection string to the MongoDB server.
    #[arg(env = "MONGODB_CONNECTION_STRING", long)]
    pub connection_string: String,

    /// The name of the database.
    #[arg(env = "DATABASE_NAME", long)]
    pub database_name: String,
}
