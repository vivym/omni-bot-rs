pub mod amqp;
pub mod commands;
pub mod db;
pub mod schemas;

use std::sync::Arc;
use mongodb::Collection;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, UserData, Error>;

pub struct UserData {
    pub video_stylizer_task_collection: Arc<Collection<schemas::VideoStylizerTaskInDB>>,
    pub video_stylizer_task_pending_channel: Arc<lapin::Channel>,
}
