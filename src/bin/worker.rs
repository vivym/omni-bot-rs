use clap::Parser;
use futures::StreamExt;
use lapin::{options::{BasicConsumeOptions, BasicAckOptions, BasicNackOptions, BasicPublishOptions}, BasicProperties, Connection, ConnectionProperties, options::QueueDeclareOptions, types::FieldTable};
use serde::{Deserialize, Serialize};
use omni_bot_rs::schemas::VideoStylizerTaskInQueue;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, env)]
    backend: Vec<String>,
    #[clap(default_value = "amqp://localhost:5672", long, env)]
    amqp_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoStylizerRequestBody {
    pub videoname: String,
    pub video_prompt: String,
    pub style_prompt: String,
    pub n_prompt: String,
    pub max_keyframe: i64,
    pub seed: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoStylizerResponseBody {
    pub output_path: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut threads = Vec::with_capacity(args.backend.len());
    for backend in args.backend {
        println!("Starting worker for backend: {}", backend);
        let amqp_uri = args.amqp_uri.clone();
        let handle = tokio::spawn(async move {
            let options = ConnectionProperties::default()
                .with_executor(tokio_executor_trait::Tokio::current())
                .with_reactor(tokio_reactor_trait::Tokio);
            let conn = Connection::connect(&amqp_uri, options).await.unwrap();

            let sending_channel = conn.create_channel().await.unwrap();
            let receiving_channel = conn.create_channel().await.unwrap();

            let options = QueueDeclareOptions {
                durable: true,
                auto_delete: true,
                ..Default::default()
            };
            sending_channel.queue_declare(
                "completedVideoStylizerTasks", options, FieldTable::default()
            ).await.unwrap();

            receiving_channel.queue_declare(
                "pendingVideoStylizerTasks", options, FieldTable::default()
            ).await.unwrap();

            let mut consumer = receiving_channel.basic_consume(
                "pendingVideoStylizerTasks",
                "worker",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await.unwrap();

            let http_client = reqwest::Client::new();

            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.expect("error in consumer");
                let _task = serde_json::from_slice::<VideoStylizerTaskInQueue>(
                    &delivery.data
                );

                if let Ok(task) = _task {
                    println!("Got task: {:?}", task);
                    match http_client.post(&backend)
                        .json(&VideoStylizerRequestBody {
                            videoname: task.src_video_url.clone(),
                            video_prompt: task.video_prompt.clone().unwrap_or_else(|| "".to_owned()),
                            style_prompt: task.style_prompt.clone(),
                            n_prompt: task.negative_prompt.clone().unwrap_or_else(|| "".to_owned()),
                            max_keyframe: if task.max_keyframes.is_none() { -1} else { task.max_keyframes.unwrap() as i64 },
                            seed: task.seed,
                        })
                        .send()
                        .await {
                        Ok(rsp) => {
                            let rsp_body = rsp.json::<VideoStylizerResponseBody>().await;
                            let (status, result) = match rsp_body {
                                Ok(rsp_body) => ("completed".to_owned(), rsp_body.output_path.clone()),
                                Err(e) => ("failed".to_owned(), format!("{:?}", e)),
                            };
                            let task = task.with_result(status, result);
                            let payload = serde_json::to_vec(&task).unwrap();
                            sending_channel.basic_publish(
                                "",
                                "completedVideoStylizerTasks",
                                BasicPublishOptions::default(),
                                &payload,
                                BasicProperties::default(),
                            ).await.unwrap().await.unwrap();
                            delivery.ack(BasicAckOptions::default()).await.expect("ack");
                        },
                        Err(e) => {
                            eprintln!("Failed to send task to backend: {:?}", e);
                            delivery.nack(BasicNackOptions::default()).await.expect("nack");
                        }
                    }
                } else {
                    println!("Failed to deserialize task.");
                    delivery.ack(BasicAckOptions::default()).await.expect("ack");
                }
            }
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.await.unwrap();
    }
}
