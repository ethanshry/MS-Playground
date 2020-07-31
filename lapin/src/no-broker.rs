/*
#[tokio::main]
async fn main() -> Result<()> {
    let addr: String =
        std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672".into());
    let conn = Connection::connect(&addr, ConnectionProperties::default().with_tokio()).await?;
    let channel_prod = conn.create_channel().await?;
    let channel_cons: lapin::Channel = conn.create_channel().await?;

    let queue = channel_prod
        .queue_declare(
            "hello",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    println!("Declared queue {:?}", queue);

    tokio::spawn(async move {
        let conn = Connection::connect(&addr, ConnectionProperties::default().with_tokio())
            .await
            .expect("Cannot connect");
        let channel_cons: lapin::Channel =
            conn.create_channel().await.expect("cannot build channel");
        let mut consumer: lapin::Consumer = channel_cons
            .basic_consume(
                "hello",
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("cannot create consumer");

        while let Some(delivery) = consumer.next().await {
            let (channel, delivery) = delivery.expect("error in consumer");
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await
                .expect("ack");
            println!("{:?}", delivery)
        }
    });

    loop {
        let confirm = channel_prod
            .basic_publish(
                "",
                "hello",
                BasicPublishOptions::default(),
                b"TALK TO ME".to_vec(),
                BasicProperties::default(),
            )
            .await?
            .await?;
        assert_eq!(confirm, Confirmation::NotRequested);
    }

    // Rest of your program
    return Ok(());
}
*/
