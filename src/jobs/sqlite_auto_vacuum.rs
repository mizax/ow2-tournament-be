use std::sync::Arc;
use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use crate::dal::Dal;

pub async fn register_sqlite_auto_vacuum_job(
    scheduler: &JobScheduler,
    dal_src: Arc<Dal>,
) -> Result<(), JobSchedulerError> {
    scheduler.add(
        Job::new_async("44 * * * * *", move |uuid, _l| {
            let dal = dal_src.clone();
            Box::pin(async move {
                info!("Running SQLite autovacuum job: {:?}", uuid);
                if let Err(e) = dal.autovacuum().await {
                    error!("SQLite autovacuum job failed: {:?}", uuid);
                    error!("{:?}", e);
                }
                info!("SQLite autovacuum job finished: {:?}", uuid);
            })
        })?
    ).await?;

    Ok(())
}
