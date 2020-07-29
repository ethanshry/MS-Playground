use async_trait::async_trait;
use futures_util::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Result,
};
use std::iter::FromIterator;
use std::str::FromStr;
use std::string::ToString;
use tokio_amqp::*;

/// The interface between Kraken and RabbitMQ
pub struct RabbitBroker {
    /// Connection to the Rabbit Instance (Should be one per device)
    pub conn: lapin::Connection,
}

impl RabbitBroker {
    /// Creates a new RabbitBroker, returns None if no rabbit connection could be made
    /// This indicates that the machine needs to spin up a rabbit instance
    pub async fn new(addr: &str) -> Option<RabbitBroker> {
        let conn = Connection::connect(&addr, ConnectionProperties::default().with_tokio()).await;
        match conn {
            Ok(c) => Some(RabbitBroker { conn: c }),
            Err(e) => {
                println!("Error establishing conn: {:?}", e);
                None
            }
        }
    }

    /// Declares a queue to which rabbitmq messages can be published
    pub async fn declare_queue(&self, queue_label: &str) -> bool {
        // Channels are cheap, and de-allocate when out of scope
        let chan = &self
            .conn
            .create_channel()
            .await
            .expect("Could not create rabbit channel for queue declaration");
        match chan
            .queue_declare(
                queue_label,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
        {
            Ok(_) => true,
            Err(_) => false, // Don't worry about faliures right now
        }
    }

    /// Gets a new publisher for the existing broker channel. This should happen on the thread it is operating on
    pub async fn get_channel(&self) -> lapin::Channel {
        self.conn
            .create_channel()
            .await
            .expect("Could not create rabbit channel via connection")
    }

    /*
    /// Gets a consumer of a queue
    pub async fn get_consumer(
        &self,
        queue_label: &str,
        consumer_tag: &str,
    ) -> (lapin::Channel, lapin::Consumer) {
        let chan: lapin::Channel = self
            .conn
            .create_channel()
            .await
            .expect("cannot build channel");
        chan.confirm_select(ConfirmSelectOptions::default())
            .await
            .expect("Cannot confirm options");

        (
            chan,
            chan.basic_consume(
                queue_label,
                consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("cannot create consumer"),
        )
    }
    */

    /// Converts a queue into a consumer
    pub async fn get_consumer(
        channel: &lapin::Channel,
        queue_label: &str,
        consumer_tag: &str,
    ) -> lapin::Consumer {
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .expect("Cannot confirm options");
        channel
            .basic_consume(
                queue_label,
                consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("cannot create consumer")
    }

    pub async fn ack(channel: &lapin::Channel, tag: u64) -> () {
        channel
            .basic_ack(tag, BasicAckOptions::default())
            .await
            .expect("ack")
    }
}

#[async_trait]
/// Trait to pack system identifier and data into a vector to be sent by rabbit,
/// and unpack a Vec<u8> to a string for easy processing
pub trait RabbitMessage<T> {
    fn build_message(&self) -> Vec<u8>;
    fn deconstruct_message(packet_data: &Vec<u8>) -> (String, T);
    async fn send(&self, channel: &lapin::Channel, tag: &str) -> bool
    where
        T: 'async_trait,
    {
        match channel
            .basic_publish(
                "",
                tag,
                BasicPublishOptions::default(),
                self.build_message(),
                BasicProperties::default(),
            )
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

// TODO build in default send functions here

// TODO implement format
pub struct SysinfoMessage {
    pub system_identifier: String,
    pub ram_free: u64,
    pub ram_used: u64,
    pub uptime: u64,
    pub load_avg_5: f32,
}

impl SysinfoMessage {
    pub fn new(system_identifier: &str) -> SysinfoMessage {
        SysinfoMessage {
            system_identifier: system_identifier.to_owned(),
            ram_free: 0,
            ram_used: 0,
            uptime: 0,
            load_avg_5: 0.0,
        }
    }

    pub fn update_message(&mut self, ram_free: u64, ram_used: u64, uptime: u64, load_avg_5: f32) {
        self.ram_free = ram_free;
        self.ram_used = ram_used;
        self.uptime = uptime;
        self.load_avg_5 = load_avg_5;
    }
}

impl RabbitMessage<SysinfoMessage> for SysinfoMessage {
    fn build_message(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}",
            self.system_identifier, self.ram_free, self.ram_used, self.uptime, self.load_avg_5
        )
        .as_bytes()
        .to_vec()
    }
    fn deconstruct_message(packet_data: &Vec<u8>) -> (String, SysinfoMessage) {
        let res = Vec::from_iter(
            String::from_utf8_lossy(packet_data)
                .split("|")
                .map(|s| s.to_string()),
        );

        let mut msg = SysinfoMessage::new(res.get(0).unwrap());

        if res.len() == 5 {
            msg.update_message(
                res.get(1).unwrap().parse::<u64>().unwrap(),
                res.get(2).unwrap().parse::<u64>().unwrap(),
                res.get(3).unwrap().parse::<u64>().unwrap(),
                res.get(4).unwrap().parse::<f32>().unwrap(),
            );
        }

        (res[0].clone(), msg)
    }
}
