mod sqlite_auto_vacuum;

use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, JobSchedulerError};
use crate::Dal;

pub async fn init_scheduler(
    dal: Arc<Dal>,
) -> Result<(), JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;
    
    sqlite_auto_vacuum::register_sqlite_auto_vacuum_job(&scheduler, dal.clone()).await?;

    scheduler.shutdown_on_ctrl_c();
    
    scheduler.start().await
}
