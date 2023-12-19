use lapin::{Connection, ConnectionProperties, options::QueueDeclareOptions, types::FieldTable};

pub async fn setup_amqp(amqp_uri: String) -> (lapin::Channel, lapin::Channel) {
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
        "pendingVideoStylizerTasks", options, FieldTable::default()
    ).await.unwrap();

    receiving_channel.queue_declare(
        "completedVideoStylizerTasks", options, FieldTable::default()
    ).await.unwrap();

    (sending_channel, receiving_channel)
}
