use async_std::stream::StreamExt;
use futures::TryStreamExt;
use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use mongodb::bson::{Bson, Document};
use mongodb::options::{FindOneOptions, FindOptions, UpdateOptions};
use mongodb::{
    bson,
    bson::doc,
    options::{ClientOptions, ServerApi, ServerApiVersion},
    Client, Collection,
};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::database::mongodb::config::MongoDbConfig;
use crate::database::Database;
use crate::jobs::types::{JobItem, JobStatus, JobType};

pub mod config;

pub struct MongoDb {
    client: Client,
}

impl MongoDb {
    pub async fn new(config: MongoDbConfig) -> Self {
        let mut client_options = ClientOptions::parse(config.url).await.expect("Failed to parse MongoDB Url");
        // Set the server_api field of the client_options object to set the version of the Stable API on the
        // client
        let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
        client_options.server_api = Some(server_api);
        // Get a handle to the cluster
        let client = Client::with_options(client_options).expect("Failed to create MongoDB client");
        // Ping the server to see if you can connect to the cluster
        client.database("admin").run_command(doc! {"ping": 1}, None).await.expect("Failed to ping MongoDB deployment");
        println!("Pinged your deployment. You successfully connected to MongoDB!");

        MongoDb { client }
    }

    /// Mongodb client uses Arc internally, reducing the cost of clone.
    /// Directly using clone is not recommended for libraries not using Arc internally.
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    fn get_job_collection(&self) -> Collection<JobItem> {
        self.client.database("orchestrator").collection("jobs")
    }

    /// Updates the job in the database optimistically. This means that the job is updated only if
    /// the version of the job in the database is the same as the version of the job passed in.
    /// If the version is different, the update fails.
    async fn update_job_optimistically(&self, current_job: &JobItem, update: Document) -> Result<()> {
        let filter = doc! {
            "id": current_job.id,
            "version": current_job.version,
        };
        let options = UpdateOptions::builder().upsert(false).build();
        let result = self.get_job_collection().update_one(filter, update, options).await?;
        if result.modified_count == 0 {
            return Err(eyre!("Failed to update job. Job version is likely outdated"));
        }
        Ok(())
    }
}

#[async_trait]
impl Database for MongoDb {
    async fn create_job(&self, job: JobItem) -> Result<JobItem> {
        self.get_job_collection().insert_one(&job, None).await?;
        Ok(job)
    }

    async fn get_job_by_id(&self, id: Uuid) -> Result<Option<JobItem>> {
        let filter = doc! {
            "id":  id
        };
        Ok(self.get_job_collection().find_one(filter, None).await?)
    }

    async fn get_job_by_internal_id_and_type(&self, internal_id: &str, job_type: &JobType) -> Result<Option<JobItem>> {
        let filter = doc! {
            "internal_id": internal_id,
            "job_type": mongodb::bson::to_bson(&job_type)?,
        };
        Ok(self.get_job_collection().find_one(filter, None).await?)
    }

    async fn update_job(&self, job: &JobItem) -> Result<()> {
        let job_doc = bson::to_document(job)?;
        let update = doc! {
            "$set": job_doc
        };
        self.update_job_optimistically(job, update).await?;
        Ok(())
    }

    async fn update_job_status(&self, job: &JobItem, new_status: JobStatus) -> Result<()> {
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&new_status)?,
            }
        };
        self.update_job_optimistically(job, update).await?;
        Ok(())
    }

    async fn update_metadata(&self, job: &JobItem, metadata: HashMap<String, String>) -> Result<()> {
        let update = doc! {
            "$set": {
                "metadata":  mongodb::bson::to_document(&metadata)?
            }
        };
        self.update_job_optimistically(job, update).await?;
        Ok(())
    }

    async fn get_latest_job_by_type(&self, job_type: JobType) -> Result<Option<JobItem>> {
        let filter = doc! {
            "job_type": mongodb::bson::to_bson(&job_type)?,
        };
        let find_options = FindOneOptions::builder().sort(doc! { "internal_id": -1 }).build();
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
                    Err(e) => eprintln!("Failed to deserialize JobItem: {:?}", e),
                },
                Err(e) => eprintln!("Error retrieving document: {:?}", e),
            }
        }

        Ok(vec_jobs)
    }

    async fn get_latest_job_by_type_and_status(
        &self,
        job_type: JobType,
        job_status: JobStatus,
    ) -> Result<Option<JobItem>> {
        let filter = doc! {
            "job_type": bson::to_bson(&job_type)?,
            "job_status": bson::to_bson(&job_status)?
        };
        let find_options = FindOneOptions::builder().sort(doc! { "internal_id": -1 }).build();

        Ok(self.get_job_collection().find_one(filter, find_options).await?)
    }

    async fn get_jobs_after_internal_id_by_job_type(
        &self,
        job_type: JobType,
        job_status: JobStatus,
        internal_id: String,
    ) -> Result<Vec<JobItem>> {
        let filter = doc! {
            "job_type": bson::to_bson(&job_type)?,
            "job_status": bson::to_bson(&job_status)?,
            "internal_id": { "$gt": internal_id }
        };

        let jobs = self.get_job_collection().find(filter, None).await?.try_collect().await?;

        Ok(jobs)
    }

    async fn get_jobs_by_statuses(&self, job_status: Vec<JobStatus>, limit: Option<i64>) -> Result<Vec<JobItem>> {
        let filter = doc! {
            "job_status": {
                // TODO: Check that the conversion leads to valid output!
                "$in": job_status.iter().map(|status| bson::to_bson(status).unwrap_or(Bson::Null)).collect::<Vec<Bson>>()
            }
        };

        let find_options = limit.map(|val| FindOptions::builder().limit(Some(val)).build());

        let jobs = self.get_job_collection().find(filter, find_options).await?.try_collect().await?;

        Ok(jobs)
    }
}

impl Serialize for JobItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("JobItem", 7)?;
        let binary = bson::Binary { subtype: bson::spec::BinarySubtype::Uuid, bytes: self.id.as_bytes().to_vec() };
        state.serialize_field("id", &binary)?;
        state.serialize_field("internal_id", &self.internal_id)?;
        state.serialize_field("job_type", &self.job_type)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("external_id", &self.external_id)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.serialize_field("version", &self.version)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for JobItem {
    fn deserialize<D>(deserializer: D) -> Result<JobItem, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct JobItemVisitor;

        impl<'de> Visitor<'de> for JobItemVisitor {
            type Value = JobItem;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct JobItem")
            }

            fn visit_map<V>(self, mut map: V) -> Result<JobItem, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut internal_id = None;
                let mut job_type = None;
                let mut status = None;
                let mut external_id = None;
                let mut metadata = None;
                let mut version = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "id" => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            // Deserialize UUID from BSON Binary
                            let binary: bson::Binary = map.next_value()?;
                            if binary.subtype != bson::spec::BinarySubtype::Uuid {
                                return Err(de::Error::custom("expected UUID binary subtype"));
                            }
                            id = Some(Uuid::from_slice(&binary.bytes).map_err(de::Error::custom)?);
                        }
                        "internal_id" => {
                            if internal_id.is_some() {
                                return Err(de::Error::duplicate_field("internal_id"));
                            }
                            internal_id = Some(map.next_value()?);
                        }
                        "job_type" => {
                            if job_type.is_some() {
                                return Err(de::Error::duplicate_field("job_type"));
                            }
                            job_type = Some(map.next_value()?);
                        }
                        "status" => {
                            if status.is_some() {
                                return Err(de::Error::duplicate_field("status"));
                            }
                            status = Some(map.next_value()?);
                        }
                        "external_id" => {
                            if external_id.is_some() {
                                return Err(de::Error::duplicate_field("external_id"));
                            }
                            external_id = Some(map.next_value()?);
                        }
                        "metadata" => {
                            if metadata.is_some() {
                                return Err(de::Error::duplicate_field("metadata"));
                            }
                            metadata = Some(map.next_value()?);
                        }
                        "version" => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                        "_id" => {
                            // Ignore the MongoDB autogenerated _id field
                            let _id: bson::oid::ObjectId = map.next_value()?;
                        }
                        _ => return Err(de::Error::unknown_field(key, FIELDS)),
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let internal_id = internal_id.ok_or_else(|| de::Error::missing_field("internal_id"))?;
                let job_type = job_type.ok_or_else(|| de::Error::missing_field("job_type"))?;
                let status = status.ok_or_else(|| de::Error::missing_field("status"))?;
                let external_id = external_id.ok_or_else(|| de::Error::missing_field("external_id"))?;
                let metadata = metadata.ok_or_else(|| de::Error::missing_field("metadata"))?;
                let version = version.ok_or_else(|| de::Error::missing_field("version"))?;

                Ok(JobItem { id, internal_id, job_type, status, external_id, metadata, version })
            }
        }

        const FIELDS: &[&str] =
            &["id", "internal_id", "job_type", "status", "external_id", "metadata", "version", "_id"];

        deserializer.deserialize_struct("JobItem", FIELDS, JobItemVisitor)
    }
}
