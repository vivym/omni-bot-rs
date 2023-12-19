use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoStylizerTaskCreation {
    pub user_id: u64,
    pub channel_id: u64,
    pub src_video_url: String,
    pub video_prompt: Option<String>,
    pub style_prompt: String,
    pub negative_prompt: Option<String>,
    pub max_keyframes: Option<u64>,
    pub seed: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoStylizerTaskInQueue {
    pub task_id: String,
    pub user_id: u64,
    pub channel_id: u64,
    pub src_video_url: String,
    pub video_prompt: Option<String>,
    pub style_prompt: String,
    pub negative_prompt: Option<String>,
    pub max_keyframes: Option<u64>,
    pub seed: u64,
    pub status: String,
    pub result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoStylizerTaskInDB {
    pub user_id: u64,
    pub channel_id: u64,
    pub src_video_url: String,
    pub video_prompt: Option<String>,
    pub style_prompt: String,
    pub negative_prompt: Option<String>,
    pub max_keyframes: Option<u64>,
    pub seed: u64,
    pub status: String,
    pub result: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl VideoStylizerTaskCreation {
    pub fn with_task_id(self, task_id: String) -> VideoStylizerTaskInQueue {
        VideoStylizerTaskInQueue {
            task_id,
            user_id: self.user_id,
            channel_id: self.channel_id,
            src_video_url: self.src_video_url,
            video_prompt: self.video_prompt,
            style_prompt: self.style_prompt,
            negative_prompt: self.negative_prompt,
            max_keyframes: self.max_keyframes,
            seed: self.seed,
            status: "pending".to_owned(),
            result: None,
        }
    }
}

impl Into<VideoStylizerTaskInDB> for VideoStylizerTaskCreation {
    fn into(self) -> VideoStylizerTaskInDB {
        VideoStylizerTaskInDB {
            user_id: self.user_id,
            channel_id: self.channel_id,
            src_video_url: self.src_video_url,
            video_prompt: self.video_prompt,
            style_prompt: self.style_prompt,
            negative_prompt: self.negative_prompt,
            max_keyframes: self.max_keyframes,
            seed: self.seed,
            status: "pending".to_owned(),
            result: None,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
        }
    }
}

impl Into<VideoStylizerTaskInDB> for VideoStylizerTaskInQueue {
    fn into(self) -> VideoStylizerTaskInDB {
        VideoStylizerTaskInDB {
            user_id: self.user_id,
            channel_id: self.channel_id,
            src_video_url: self.src_video_url,
            video_prompt: self.video_prompt,
            style_prompt: self.style_prompt,
            negative_prompt: self.negative_prompt,
            max_keyframes: self.max_keyframes,
            seed: self.seed,
            status: self.status,
            result: self.result,
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
        }
    }
}

impl VideoStylizerTaskInQueue {
    pub fn with_result(self, status: String, result: String) -> VideoStylizerTaskInQueue {
        VideoStylizerTaskInQueue {
            status,
            result: Some(result),
            ..self
        }
    }
}
