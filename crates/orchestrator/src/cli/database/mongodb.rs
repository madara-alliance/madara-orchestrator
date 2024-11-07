use clap::Args;

/// Parameters used to config MongoDB.
#[derive(Debug, Clone, Args)]
#[group(requires_all = ["connection_url", "database_name"])]
pub struct MongoDBCliArgs {
    /// Use the MongoDB client
    #[arg(long)]
    pub mongodb: bool,

    /// The connection string to the MongoDB server.
    #[arg(env = "MONGODB_CONNECTION_URL", long, default_value = Some("mongodb://localhost:27017"))]
    pub connection_url: Option<String>,

    /// The name of the database.
    #[arg(env = "DATABASE_NAME", long, default_value = Some("orchestrator"))]
    pub database_name: Option<String>,
}
