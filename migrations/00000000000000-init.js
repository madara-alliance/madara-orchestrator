module.exports = {
    async up(db) {
      // Create indexes for the 'jobs' collection
      await db.collection('jobs').createIndexes([
        { key: { "id": 1 } },
        { key: { "internal_id": 1, "job_type": 1 } },
        { key: { "job_type": 1, "status": 1, "internal_id": -1 } },
        { key: { "status": 1 } }
      ]);
    },
  
    async down(db) {
      // Drop indexes for the 'jobs' collection
      await db.collection('jobs').dropIndex("id_1");
      await db.collection('jobs').dropIndex("internal_id_1_job_type_1");
      await db.collection('jobs').dropIndex("job_type_1_status_1_internal_id_-1");
      await db.collection('jobs').dropIndex("status_1");
    }
  };