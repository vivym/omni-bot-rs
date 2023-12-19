use crate::schemas::VideoStylizerTaskInDB;
use mongodb::{Client, Collection, options::ClientOptions};

pub async fn setup_db(uri: String) -> (Collection<VideoStylizerTaskInDB>,) {
    let mut client_options = ClientOptions::parse(uri).await.unwrap();

    client_options.app_name = Some("OmniBot".to_string());

    let client = Client::with_options(client_options).unwrap();

    let db = client.database("OmniAI");

    let video_stylizer_task_collection = db.collection::<VideoStylizerTaskInDB>(
        "video_stylizer_task"
    );

    (video_stylizer_task_collection,)
}
