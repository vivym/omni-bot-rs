use mongodb::bson::doc;
use omni_bot_rs::{UserData, Error, amqp, commands, db, schemas};
use futures::StreamExt;
use lapin::{options::{BasicConsumeOptions, BasicAckOptions}, types::FieldTable};
use poise::serenity_prelude::{self as serenity, ChannelId, Channel, AttachmentType};
use tokio::sync::mpsc;
use std::{env, sync::Arc, time::Duration};

async fn on_error(error: poise::FrameworkError<'_, UserData, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => {
            eprintln!("Failed to setup bot: {:?}", error);
        }
        poise::FrameworkError::Command { error, ctx, .. } => {
            eprintln!("Error in command `{}`: {:?}", ctx.command().name, error);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                eprintln!("Error in error handler: {:?}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mongo_uri = env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017".to_owned());
    let (video_stylizer_task_collection,) = db::setup_db(mongo_uri).await;
    let video_stylizer_task_collection_clone = video_stylizer_task_collection.clone();

    let amqp_uri = env::var("AMQP_URI").unwrap_or("amqp://localhost:5672".to_owned());
    let (sending_channel, receiving_channel) = amqp::setup_amqp(amqp_uri).await;

    let options = poise::FrameworkOptions {
        commands: vec![commands::help(), commands::video_to_video::video_stylizer()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600),)),
            ..Default::default()
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!("Got an event in event handler: {:?}", event.name());
                Ok(())
            })
        },
        ..Default::default()
    };

    let (ctx_sender, mut ctx_receiver) = mpsc::channel(1);
    let framework = poise::Framework::builder()
        .token(
            env::var("DISCORD_TOKEN")
                .expect("Expected a token in the environment")
        )
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                ctx_sender.send(ctx.clone()).await.unwrap();
                Ok(UserData {
                    video_stylizer_task_collection: Arc::new(video_stylizer_task_collection),
                    video_stylizer_task_pending_channel: Arc::new(sending_channel),
                })
            })
        })
        .options(options)
        .intents(serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT)
        .build().await.unwrap();

    let bot_server = async {
        framework
            .start()
            .await
            .unwrap();
    };

    let task_callback = async {
        let ctx = ctx_receiver.recv().await.unwrap();

        let mut consumer = receiving_channel.basic_consume(
            "completedVideoStylizerTasks",
            "bot",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await.unwrap();

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.expect("error in consumer");
            let _task = serde_json::from_slice::<schemas::VideoStylizerTaskInQueue>(
                &delivery.data
            );
            if let Ok(task) = _task {
                println!("Got task: {:?}", task);
                video_stylizer_task_collection_clone.update_one(
                    doc! {"_id": task.task_id.clone()},
                    doc! {"$set": {"status": task.status.clone(), "result": task.result.clone().unwrap()}},
                    None
                ).await.unwrap();

                match task.status.as_str() {
                    "completed" => {
                        if task.result.is_none() {
                            eprintln!("Task result is none: {:?}", task);
                            continue;
                        }

                        let dst_video_path = task.result.unwrap();

                        if !tokio::fs::try_exists(&dst_video_path).await.unwrap() {
                            eprintln!("File does not exist: {:?}", dst_video_path);
                            continue;
                        }

                        let dst_video_name = dst_video_path.split("/").last().unwrap().to_owned();
                        let dst_video_file = tokio::fs::File::open(dst_video_path).await.unwrap();

                        let channel = ChannelId(task.channel_id).to_channel(&ctx).await;
                        if let Ok(Channel::Guild(channel)) = channel {
                            let mut responses = Vec::with_capacity(6);
                            responses.push(
                                format!(
                                    "New generation from <@{}>:\nTask: **Video Stylization**\nTask ID: {}",
                                    task.user_id,
                                    task.task_id,
                                )
                            );

                            if let Some(video_prompt) = task.video_prompt {
                                responses.push(format!("Video Prompt: {}", video_prompt));
                            }

                            responses.push(format!("Style Prompt: {}", task.style_prompt));

                            if let Some(negative_prompt) = task.negative_prompt {
                                responses.push(format!("Negative Prompt: {}", negative_prompt));
                            }

                            if let Some(max_keyframes) = task.max_keyframes {
                                responses.push(format!("Max Keyframes: {}", max_keyframes));
                            }

                            responses.push(format!("Seed: {}", task.seed));

                            let response = responses.join("\n");

                            channel.send_files(
                                &ctx,
                                [AttachmentType::File { file: &dst_video_file, filename: dst_video_name }],
                                |m| m.content(response),
                            ).await.unwrap();
                        } else {
                            eprintln!("Failed to get channel. {:?}", channel);
                        }
                    },
                    "failed" => {
                        let channel = ChannelId(task.channel_id).to_channel(&ctx).await;
                        if let Ok(Channel::Guild(channel)) = channel {
                            channel.say(
                                &ctx,
                                format!(
                                    "> Failed to stylize your video. <@{}> Error: {:?}", task.user_id, task.result.unwrap()
                                ),
                            ).await.unwrap();
                        } else {
                            eprintln!("Failed to get channel. {:?}", channel);
                        }
                    },
                    _ => {
                        eprintln!("Invalid task status: {:?}", task);
                    }
                }
            } else {
                eprintln!("Failed to deserialize task.");
            }
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
        }
    };

    tokio::select! {
        _ = bot_server => {},
        _ = task_callback => {},
    }
}
