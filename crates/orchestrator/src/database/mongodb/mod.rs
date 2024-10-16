use ::utils::settings::Settings;
use async_std::stream::StreamExt;
use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use futures::TryStreamExt;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{ClientOptions, FindOneOptions, FindOptions, ServerApi, ServerApiVersion, UpdateOptions};
use mongodb::{bson, Client, Collection};
use utils::ToDocument;
use uuid::Uuid;

use crate::database::mongodb::config::MongoDbConfig;
use crate::database::{Database, DatabaseConfig};
use crate::jobs::types::{JobItem, JobItemUpdates, JobStatus, JobType};
use crate::jobs::JobError;

pub mod config;
mod utils;

pub struct MongoDb {
    client: Client,
    database_name: String,
}

impl MongoDb {
    pub async fn new_with_settings(settings: &impl Settings) -> Self {
        let mongo_db_settings = MongoDbConfig::new_with_settings(settings);
        let mut client_options =
            ClientOptions::parse(mongo_db_settings.url).await.expect("Failed to parse MongoDB Url");
        // Set the server_api field of the client_options object to set the version of the Stable API on the
        // client
        let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
        client_options.server_api = Some(server_api);
        // Get a handle to the cluster
        let client = Client::with_options(client_options).expect("Failed to create MongoDB client");
        // Ping the server to see if you can connect to the cluster
        client
            .database("orchestrator")
            .run_command(doc! {"ping": 1}, None)
            .await
            .expect("Failed to ping MongoDB deployment");
        tracing::debug!("Pinged your deployment. You successfully connected to MongoDB!");

        Self { client, database_name: mongo_db_settings.database_name }
    }

    /// Mongodb client uses Arc internally, reducing the cost of clone.
    /// Directly using clone is not recommended for libraries not using Arc internally.
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    fn get_job_collection(&self) -> Collection<JobItem> {
        self.client.database(&self.database_name).collection("jobs")
    }
}

#[async_trait]
impl Database for MongoDb {
    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn create_job(&self, job: JobItem) -> Result<JobItem, JobError> {
        let options = UpdateOptions::builder().upsert(true).build();

        let updates = job.to_document().map_err(|e| JobError::Other(e.into()))?;
        let job_type =
            updates.get("job_type").ok_or(eyre!("Job type not found")).map_err(|e| JobError::Other(e.into()))?;
        let internal_id =
            updates.get("internal_id").ok_or(eyre!("Internal ID not found")).map_err(|e| JobError::Other(e.into()))?;

        // Filter using only two fields
        let filter = doc! {
            "job_type": job_type.clone(),
            "internal_id": internal_id.clone()
        };

        let updates = doc! {
            // only set when the document is inserted for the first time
            "$setOnInsert": updates
        };

        let result = self
            .get_job_collection()
            .update_one(filter, updates, options)
            .await
            .map_err(|e| JobError::Other(e.to_string().into()))?;

        if result.matched_count == 0 {
            Ok(job)
        } else {
            Err(JobError::JobAlreadyExists { internal_id: job.internal_id, job_type: job.job_type })
        }
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_job_by_id(&self, id: Uuid) -> Result<Option<JobItem>> {
        let filter = doc! {
            "id":  id
        };
        tracing::debug!(job_id = %id, category = "db_call", "Fetched job by ID");
        Ok(self.get_job_collection().find_one(filter, None).await?)
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_job_by_internal_id_and_type(&self, internal_id: &str, job_type: &JobType) -> Result<Option<JobItem>> {
        let filter = doc! {
            "internal_id": internal_id,
            "job_type": mongodb::bson::to_bson(&job_type)?,
        };
        tracing::debug!(internal_id = %internal_id, job_type = ?job_type, category = "db_call", "Fetched job by internal ID and type");
        Ok(self.get_job_collection().find_one(filter, None).await?)
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn update_job(&self, current_job: &JobItem, updates: JobItemUpdates) -> Result<()> {
        // Filters to search for the job
        let filter = doc! {
            "id": current_job.id,
            "version": current_job.version,
        };
        let options = UpdateOptions::builder().upsert(false).build();

        let mut updates = updates.to_document()?;

        // remove null values from the updates
        let mut non_null_updates = Document::new();
        updates.iter_mut().for_each(|(k, v)| {
            if v != &Bson::Null {
                non_null_updates.insert(k, v);
            }
        });

        // throw an error if there's no field to be updated
        if non_null_updates.is_empty() {
            return Err(eyre!("No field to be updated, likely a false call"));
        }

        // Add additional fields that are always updated
        non_null_updates.insert("version", Bson::Int32(current_job.version + 1));
        non_null_updates.insert("updated_at", Bson::DateTime(Utc::now().round_subsecs(0).into()));

        let update = doc! {
            "$set": non_null_updates
        };

        let result = self.get_job_collection().update_one(filter, update, options).await?;
        if result.modified_count == 0 {
            tracing::warn!(job_id = %current_job.id, category = "db_call", "Failed to update job. Job version is likely outdated");
            return Err(eyre!("Failed to update job. Job version is likely outdated"));
        }

        tracing::debug!(job_id = %current_job.id, category = "db_call", "Job updated successfully");
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_latest_job_by_type(&self, job_type: JobType) -> Result<Option<JobItem>> {
        let filter = doc! {
            "job_type": mongodb::bson::to_bson(&job_type)?,
        };
        let find_options = FindOneOptions::builder().sort(doc! { "created_at": -1 }).build();
        tracing::debug!(job_type = ?job_type, category = "db_call", "Fetching latest job by type");
        Ok(self.get_job_collection().find_one(filter, find_options).await?)
    }

    /// function to get jobs that don't have a successor job.
    ///
    /// `job_a_type` : Type of job that we need to get that doesn't have any successor.
    ///
    /// `job_a_status` : Status of job A.
    ///
    /// `job_b_type` : Type of job that we need to have as a successor for Job A.
    ///
    /// `job_b_status` : Status of job B which we want to check with.
    ///
    /// Eg :
    ///
    /// Getting SNOS jobs that do not have a successive proving job initiated yet.
    ///
    /// job_a_type : SnosRun
    ///
    /// job_a_status : Completed
    ///
    /// job_b_type : ProofCreation
    ///
    /// TODO : For now Job B status implementation is pending so we can pass None
    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_jobs_without_successor(
        &self,
        job_a_type: JobType,
        job_a_status: JobStatus,
        job_b_type: JobType,
    ) -> Result<Vec<JobItem>> {
        // Convert enums to Bson strings
        let job_a_type_bson = Bson::String(format!("{:?}", job_a_type));
        let job_a_status_bson = Bson::String(format!("{:?}", job_a_status));
        let job_b_type_bson = Bson::String(format!("{:?}", job_b_type));

        // TODO :
        // implement job_b_status here in the pipeline

        // Construct the initial pipeline
        let pipeline = vec![
            // Stage 1: Match job_a_type with job_a_status
            doc! {
                "$match": {
                    "job_type": job_a_type_bson,
                    "status": job_a_status_bson,
                }
            },
            // Stage 2: Lookup to find corresponding job_b_type jobs
            doc! {
                "$lookup": {
                    "from": "jobs",
                    "let": { "internal_id": "$internal_id" },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        { "$eq": ["$job_type", job_b_type_bson] },
                                        // Conditionally match job_b_status if provided
                                        { "$eq": ["$internal_id", "$$internal_id"] }
                                    ]
                                }
                            }
                        },
                        // TODO : Job B status code :
                        // // Add status matching if job_b_status is provided
                        // if let Some(status) = job_b_status {
                        //     doc! {
                        //         "$match": {
                        //             "$expr": { "$eq": ["$status", status] }
                        //         }
                        //     }
                        // } else {
                        //     doc! {}
                        // }
                    // ].into_iter().filter(|d| !d.is_empty()).collect::<Vec<_>>(),
                    ],
                    "as": "successor_jobs"
                }
            },
            // Stage 3: Filter out job_a_type jobs that have corresponding job_b_type jobs
            doc! {
                "$match": {
                    "successor_jobs": { "$eq": [] }
                }
            },
        ];
        // TODO : Job B status code :
        // // Conditionally add status matching for job_b_status
        // if let Some(status) = job_b_status {
        //     let job_b_status_bson = Bson::String(format!("{:?}", status));
        //
        //     // Access the "$lookup" stage in the pipeline and modify the "pipeline" array inside it
        //     if let Ok(lookup_stage) = pipeline[1].get_document_mut("pipeline") {
        //         if let Ok(lookup_pipeline) = lookup_stage.get_array_mut(0) {
        //             lookup_pipeline.push(Bson::Document(doc! {
        //             "$match": {
        //                 "$expr": { "$eq": ["$status", job_b_status_bson] }
        //             }
        //         }));
        //         }
        //     }
        // }

        let mut cursor = self.get_job_collection().aggregate(pipeline, None).await?;

        let mut vec_jobs: Vec<JobItem> = Vec::new();

        // Iterate over the cursor and process each document
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => match bson::from_bson(Bson::Document(document)) {
                    Ok(job_item) => vec_jobs.push(job_item),
                    Err(e) => tracing::error!(error = %e, category = "db_call", "Failed to deserialize JobItem"),
                },
                Err(e) => tracing::error!(error = %e, category = "db_call", "Error retrieving document"),
            }
        }

        tracing::debug!(job_count = vec_jobs.len(), category = "db_call", "Retrieved jobs without successor");
        Ok(vec_jobs)
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_latest_job_by_type_and_status(
        &self,
        job_type: JobType,
        job_status: JobStatus,
    ) -> Result<Option<JobItem>> {
        let filter = doc! {
            "job_type": bson::to_bson(&job_type)?,
            "status": bson::to_bson(&job_status)?
        };
        let find_options = FindOneOptions::builder().sort(doc! { "internal_id": -1 }).build();

        tracing::debug!(job_type = ?job_type, job_status = ?job_status, category = "db_call", "Fetched latest job by type and status");
        Ok(self.get_job_collection().find_one(filter, find_options).await?)
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"), ret, err)]
    async fn get_jobs_after_internal_id_by_job_type(
        &self,
        job_type: JobType,
        job_status: JobStatus,
        internal_id: String,
    ) -> Result<Vec<JobItem>> {
        let filter = doc! {
            "job_type": bson::to_bson(&job_type)?,
            "status": bson::to_bson(&job_status)?,
            "internal_id": { "$gt": internal_id.clone() }
        };
        let jobs: Vec<JobItem> = self.get_job_collection().find(filter, None).await?.try_collect().await?;
        tracing::debug!(job_type = ?job_type, job_status = ?job_status, internal_id = internal_id, category = "db_call", "Fetched jobs after internal ID by job type");
        Ok(jobs)
    }

    #[tracing::instrument(skip(self, limit), fields(function_type = "db_call"), ret, err)]
    async fn get_jobs_by_statuses(&self, job_status: Vec<JobStatus>, limit: Option<i64>) -> Result<Vec<JobItem>> {
        let filter = doc! {
            "status": {
                // TODO: Check that the conversion leads to valid output!
                "$in": job_status.iter().map(|status| bson::to_bson(status).unwrap_or(Bson::Null)).collect::<Vec<Bson>>()
            }
        };

        let find_options = limit.map(|val| FindOptions::builder().limit(Some(val)).build());

        let jobs: Vec<JobItem> = self.get_job_collection().find(filter, find_options).await?.try_collect().await?;
        tracing::debug!(job_count = jobs.len(), category = "db_call", "Retrieved jobs by statuses");
        Ok(jobs)
    }

    #[tracing::instrument(skip(self), fields(function_type = "db_call"))]
    async fn get_jobs_by_type(&self, job_type: JobType) -> Result<Vec<JobItem>> {
        let filter = doc! {
            "job_type": bson::to_bson(&job_type)?,
        };

        let jobs = self.get_job_collection().find(filter, None).await?.try_collect().await?;

        Ok(jobs)
    }
}
