mod rabbit;

#[allow(unused_imports)]
use futures_util::stream::StreamExt;
use rabbit::{RabbitBroker, RabbitMessage, SysinfoMessage};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let addr: String =
        std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672".into());
    let broker = match RabbitBroker::new(&addr).await {
        Some(b) => b,
        None => panic!("Could not establish rabbit connection"),
    };

    let QUEUE_TAG = "system_status_queue";

    broker.declare_queue(QUEUE_TAG).await;

    let producer = broker.get_channel().await;
    let consumer_channel = broker.get_channel().await;
    println!("Rabbit Pub/Cons established");

    tokio::spawn(async move {
        let mut consumer = RabbitBroker::get_consumer(&consumer_channel, QUEUE_TAG, "12345").await;
        while let Some(delivery) = consumer.next().await {
            let (channel, delivery) = delivery.expect("error in consumer");
            RabbitBroker::ack(&channel, delivery.delivery_tag).await;
            println!("{:?}", String::from_utf8_lossy(&delivery.data))
        }
        println!("Done waiting on delivery");
    });

    let mut msg = SysinfoMessage::new("1234");

    let mut cnt = 0;

    loop {
        msg.update_message(25, 75, cnt, 12.34);
        cnt += 1;
        let res = msg.send(&producer, QUEUE_TAG).await;
        println!("r {}", res);
        std::thread::sleep(std::time::Duration::new(1, 0));
        if cnt > 50 {
            break;
        }
    }

    Ok(())
}

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
