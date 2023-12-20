use crate::{Context, Error, schemas::{VideoStylizerTaskCreation, VideoStylizerTaskInDB}};
use lapin::{options::BasicPublishOptions, BasicProperties};
use poise::{serenity_prelude as serenity, ChoiceParameter};
use mongodb::results::InsertOneResult;

#[derive(ChoiceParameter)]
enum StyleChoice {
    #[name = "Chinese Painting"]
    ChinesePainting,
    #[name = "Oil Painting"]
    OilPainting,
    #[name = "Cyberpunk"]
    Cyberpunk,
    #[name = "3D Cartoon"]
    Cartoon3D,
    #[name = "Japanese Animation"]
    JapaneseAnimation,
    #[name = "Paper Art"]
    PaperArt,
    #[name = "Clay Look"]
    ClayLook,
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Video to Video",
    description_localized("en-US", "Stylize a video with a style prompt."),
    description_localized("zh-CN", "视频风格化。"),
)]
pub async fn video_stylizer(
    ctx: Context<'_>,
    #[description = "Video to stylize."]
    video: serenity::Attachment,
    #[description = "Style prompt to apply to the video."]
    style_prompt: StyleChoice,
    #[description = "Video prompt."]
    video_prompt: Option<String>,
    #[description = "Negative prompt to apply to the video."]
    negative_prompt: Option<String>,
    #[description = "Maximum number of keyframes."]
    #[min = 2]
    max_keyframes: Option<u64>,
    #[description = "Seed for the random number generator."]
    seed: Option<u64>,
) -> Result<(), Error> {
    let video_url = video.url;
    let seed = seed.unwrap_or_else(|| rand::random::<u16>() as u64);

    if video.size > 64 * 1024 * 1024 {
        let response = "> File size too large. Max file size is **64MB**.".to_owned();
        ctx.say(response).await?;
        return Ok(());
    }

    let style_prompt = match style_prompt {
        StyleChoice::ChinesePainting => "<chinese painting>",
        StyleChoice::OilPainting => "<oil painting>",
        StyleChoice::Cyberpunk => "<cyberpunk>",
        StyleChoice::Cartoon3D => "<3d cartoon>",
        StyleChoice::JapaneseAnimation => "<japanese animation>",
        StyleChoice::PaperArt => "<paper art>",
        StyleChoice::ClayLook => "<clay look>",
    };
    let style_prompt = style_prompt.to_owned();

    let task = VideoStylizerTaskCreation {
        user_id: ctx.author().id.0,
        channel_id: ctx.channel_id().0,
        src_video_url: video_url,
        video_prompt,
        style_prompt,
        negative_prompt,
        max_keyframes,
        seed,
    };

    let task_id = {
        let col = ctx.data().video_stylizer_task_collection.clone();

        let task_in_db: VideoStylizerTaskInDB = task.clone().into();
        match col.insert_one(task_in_db, None).await {
            Ok(InsertOneResult{ inserted_id, .. }) => {
                Some(inserted_id.as_object_id().unwrap().to_hex())
            },
            Err(err) => {
                let response = format!("> Failed to create video stylization task. Error: {:?}", err);
                ctx.say(response).await?;
                None
            }
        }
    };

    if task_id.is_some() {
        let task_id = task_id.unwrap();
        let response = format!(
            "> We are working on your video. We will notify you when it is ready. Task ID: **{task_id}.**"
        );
        ctx.say(response).await?;

        let channel = ctx.data().video_stylizer_task_pending_channel.clone();

        let payload = serde_json::to_vec(&task.with_task_id(task_id)).unwrap();

        channel.basic_publish(
            "",
            "pendingVideoStylizerTasks",
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        ).await?.await?;
    };

    Ok(())
}
