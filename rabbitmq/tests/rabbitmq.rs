use rabbitmq::{
    Message, PublisherContext, PublisherMode, RabbitMQBuilder, RabbitMQError, SubscriberMode,
};
use std::{env, time::Duration};
use tokio::time;
use tracing::{debug, info};

#[derive(Debug, PartialEq, Eq, Hash)]
enum MySubscriptions {
    Orders,
    Trades,
    WhateverWorkerQueue,
    BroadcastTest,
    TopicTest,
    ConcurrencyTest,
}

impl From<&'static str> for MySubscriptions {
    fn from(s: &'static str) -> Self {
        match s {
            "Orders" => Self::Orders,
            "Trades" => Self::Trades,
            "WhateverWorkerQueue" => Self::WhateverWorkerQueue,
            "BroadcastTest" => Self::BroadcastTest,
            "TopicTest" => Self::TopicTest,
            "ConcurrencyTest" => Self::ConcurrencyTest,
            _ => panic!("Unknown worker queue: {}", s),
        }
    }
}

impl From<MySubscriptions> for &'static str {
    fn from(queue: MySubscriptions) -> Self {
        match queue {
            MySubscriptions::Orders => "Orders",
            MySubscriptions::Trades => "Trades",
            MySubscriptions::WhateverWorkerQueue => "WhateverWorkerQueue",
            MySubscriptions::BroadcastTest => "BroadcastTest",
            MySubscriptions::TopicTest => "TopicTest",
            MySubscriptions::ConcurrencyTest => "ConcurrencyTest",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum MyPublishers {
    Orders,
    Trades,
    WhateverWorkerQueue,
    BroadcastTest,
    TopicTest,
    ConcurrencyTest,
}

impl From<&'static str> for MyPublishers {
    fn from(s: &'static str) -> Self {
        match s {
            "Orders" => Self::Orders,
            "Trades" => Self::Trades,
            "WhateverWorkerQueue" => Self::WhateverWorkerQueue,
            "BroadcastTest" => Self::BroadcastTest,
            "TopicTest" => Self::TopicTest,
            "ConcurrencyTest" => Self::ConcurrencyTest,
            _ => panic!("Unknown worker queue: {}", s),
        }
    }
}

impl From<MyPublishers> for &'static str {
    fn from(queue: MyPublishers) -> Self {
        match queue {
            MyPublishers::Orders => "Orders",
            MyPublishers::Trades => "Trades",
            MyPublishers::WhateverWorkerQueue => "WhateverWorkerQueue",
            MyPublishers::BroadcastTest => "BroadcastTest",
            MyPublishers::TopicTest => "TopicTest",
            MyPublishers::ConcurrencyTest => "ConcurrencyTest",
        }
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_pubsub_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::Orders, PublisherMode::PubSub)
        .subscriber(MySubscriptions::Orders, SubscriberMode::PubSub);

    let (client, server) = match builder.build().await {
        Ok((client, server)) => (client, server),
        Err(err) => {
            panic!("error {}", err)
        }
    };

    let mut client_queues = client.get_publishers();
    let mut server_queues = server.get_subscribers();

    // Test message
    let message = b"test message".to_vec();

    // Publish a message
    let publisher = client_queues
        .take_ownership((MyPublishers::Orders, PublisherMode::PubSub))
        .unwrap();
    publisher
        .publish(
            Message::from(&message),
            PublisherContext::new("req_id", None),
        )
        .unwrap();

    // Wait a bit to ensure message is processed
    time::sleep(Duration::from_millis(100)).await;

    // Receive the message
    let mut subscriber = server_queues
        .take_ownership((MySubscriptions::Orders, SubscriberMode::PubSub))
        .unwrap();

    if let Some(consumer_msg) = subscriber.receive().await {
        debug!(
            "consumer message > basic props {:?}",
            consumer_msg.basic_properties
        );
        debug!("consumer message > content {:?}", consumer_msg.content);
        assert_eq!(consumer_msg.content.as_ref().unwrap().clone(), message);
        subscriber.ack(&consumer_msg).await.unwrap();
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_broadcast_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    info!("connection string {}", connection_string);

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::BroadcastTest, PublisherMode::Broadcast)
        .subscriber(MySubscriptions::BroadcastTest, SubscriberMode::Broadcast);

    let (client, server) = match builder.build().await {
        Ok((client, server)) => (client, server),
        Err(err) => {
            panic!("error {}", err)
        }
    };

    let mut client_queues = client.get_publishers();
    let mut server_queues = server.get_subscribers();

    // Test message
    let message = b"broadcast test message".to_vec();

    // Publish a message
    let publisher = client_queues
        .take_ownership((MyPublishers::BroadcastTest, PublisherMode::Broadcast))
        .unwrap();
    publisher
        .publish(
            Message::from(&message),
            PublisherContext::new("req_id", None),
        )
        .unwrap();

    time::sleep(Duration::from_millis(200)).await;

    // Receive the message
    let mut subscriber = server_queues
        .take_ownership((MySubscriptions::BroadcastTest, SubscriberMode::Broadcast))
        .unwrap();

    if let Some(consumer_msg) = subscriber.receive().await {
        debug!(
            "consumer message > basic props {:?}",
            consumer_msg.basic_properties
        );
        debug!("consumer message > content {:?}", consumer_msg.content);
        assert_eq!(consumer_msg.content.unwrap(), message);
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_topic_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let topic = "test.topic.key".to_string();
    let routing_pattern = "test.topic.key".to_string();

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::TopicTest, PublisherMode::Topic)
        .subscriber(
            MySubscriptions::TopicTest,
            SubscriberMode::Topics {
                topics: vec![routing_pattern.clone()],
            },
        );

    let (client, server) = match builder.build().await {
        Ok((client, server)) => (client, server),
        Err(err) => {
            panic!("error {}", err)
        }
    };

    let mut client_queues = client.get_publishers();
    let mut server_queues = server.get_subscribers();

    // Test message
    let message = b"topic test message".to_vec();

    // Publish a message
    let publisher = client_queues
        .take_ownership((MyPublishers::TopicTest, PublisherMode::Topic))
        .unwrap();
    publisher
        .publish(
            Message::from(&message).with_topic(&routing_pattern),
            PublisherContext::new("req_id", None),
        )
        .unwrap();

    // Wait a bit to ensure message is processed
    time::sleep(Duration::from_millis(400)).await;

    // Receive the message
    let mut subscriber = server_queues
        .take_ownership((
            MySubscriptions::TopicTest,
            SubscriberMode::Topics {
                topics: vec![routing_pattern],
            },
        ))
        .unwrap();

    if let Some(consumer_msg) = subscriber.receive().await {
        debug!(
            "consumer message > basic props {:?}",
            consumer_msg.basic_properties
        );
        debug!("consumer message > content {:?}", consumer_msg.content);
        assert_eq!(
            consumer_msg
                .deliver
                .as_ref()
                .unwrap()
                .routing_key()
                .as_str(),
            topic.as_str()
        );
        assert_eq!(consumer_msg.content.as_ref().unwrap().clone(), message);
        subscriber.ack(&consumer_msg).await.unwrap();
    } else {
        panic!("Did not receive topic message");
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_subscriber_only_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    // Create a builder with only a subscriber
    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .subscriber(MySubscriptions::Orders, SubscriberMode::PubSub);

    // Build the server (subscriber only)
    let server = match builder.build().await {
        Ok(server) => server,
        Err(err) => {
            panic!("error {}", err)
        }
    };

    // Get the subscribers, which consumes the server
    let mut server_queues = server.get_subscribers();

    // Ensure we can get the subscriber
    let subscriber = server_queues
        .take_ownership((MySubscriptions::Orders, SubscriberMode::PubSub))
        .unwrap();

    drop(subscriber);

    // Let it run a bit to ensure no issues during shutdown
    time::sleep(Duration::from_millis(100)).await;
    // If we reach this point without hanging, the test passes
}

#[test_log::test(tokio::test)]
async fn rabbitmq_publisher_only_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    // Create a builder with only a publisher
    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::Orders, PublisherMode::PubSub)
        .publisher(MyPublishers::Trades, PublisherMode::PubSub);

    // Build the client (publisher only)
    let client = match builder.build().await {
        Ok(client) => client,
        Err(err) => {
            panic!("error {}", err)
        }
    };

    // Get the publishers, which consumes the client
    let mut client_queues = client.get_publishers();

    // Ensure we can get the publisher
    let publisher = client_queues
        .take_ownership((MyPublishers::Orders, PublisherMode::PubSub))
        .unwrap();
    let publisher2 = client_queues
        .take_ownership((MyPublishers::Trades, PublisherMode::PubSub))
        .unwrap();

    // Publish a message (even though no one is listening)
    let message = b"test message".to_vec();
    publisher
        .publish(
            Message::from(&message),
            PublisherContext::new("req_id", None),
        )
        .unwrap();

    drop(publisher);
    drop(publisher2);

    // Let it run a bit to ensure no issues during shutdown
    time::sleep(Duration::from_millis(100)).await;
    // If we reach this point without hanging, the test passes
}

#[test_log::test(tokio::test)]
async fn rabbitmq_invalid_connection_error_test() {
    // Test with an invalid connection string
    let connection_string = "amqp://invalid:invalid@nonexistenthost:5672";

    let builder = RabbitMQBuilder::new(connection_string, "TEST_APP")
        .publisher(MyPublishers::Orders, PublisherMode::PubSub)
        .subscriber(MySubscriptions::Orders, SubscriberMode::PubSub);

    // Expect connection error
    match builder.build().await {
        Ok(_) => panic!("Expected connection error, but build succeeded"),
        Err(err) => {
            match err {
                RabbitMQError::ConnectionError(_) => {
                    // This is the expected error
                    debug!("Got expected connection error: {:?}", err);
                }
                _ => panic!("Expected ConnectionError, but got: {:?}", err),
            }
        }
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_missing_topic_error_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::TopicTest, PublisherMode::Topic);

    let client = match builder.build().await {
        Ok(client) => client,
        Err(err) => panic!("Failed to build: {:?}", err),
    };

    let mut client_queues = client.get_publishers();

    // Test message
    let message = b"topic message without topic".to_vec();

    // Publish a message to a topic exchange WITHOUT specifying a topic
    let publisher = client_queues
        .take_ownership((MyPublishers::TopicTest, PublisherMode::Topic))
        .unwrap();

    // This should return a MissingTopic error
    let result = publisher.publish(
        Message::from(&message), // No topic specified
        PublisherContext::new("req_id", None),
    );

    match result {
        Ok(_) => panic!("Expected MissingTopic error, but publish succeeded"),
        Err(err) => {
            match err {
                RabbitMQError::MissingTopic => {
                    // This is the expected error
                    debug!("Got expected MissingTopic error");
                }
                _ => panic!("Expected MissingTopic, but got: {:?}", err),
            }
        }
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_not_a_queue_error_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::Orders, PublisherMode::PubSub);

    let client = match builder.build().await {
        Ok(client) => client,
        Err(err) => panic!("Failed to build: {:?}", err),
    };

    let mut client_queues = client.get_publishers();

    // Try to get a publisher that doesn't exist
    let result = client_queues.take_ownership((MyPublishers::Trades, PublisherMode::PubSub));

    match result {
        Ok(_) => panic!("Expected NotAQueue error, but take_ownership succeeded"),
        Err(err) => {
            match err {
                RabbitMQError::NotAQueue => {
                    // This is the expected error
                    debug!("Got expected NotAQueue error");
                }
                _ => panic!("Expected NotAQueue, but got: {:?}", err),
            }
        }
    }
}

#[test_log::test(tokio::test)]
async fn rabbitmq_publisher_dispatcher_concurrency_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::ConcurrencyTest, PublisherMode::PubSub)
        .subscriber(MySubscriptions::ConcurrencyTest, SubscriberMode::PubSub);

    let (client, server) = match builder.build().await {
        Ok((client, server)) => (client, server),
        Err(err) => panic!("Failed to build: {:?}", err),
    };

    let mut client_queues = client.get_publishers();
    let mut server_queues = server.get_subscribers();

    // Get the publisher
    let publisher = client_queues
        .take_ownership((MyPublishers::ConcurrencyTest, PublisherMode::PubSub))
        .unwrap();

    // Create multiple dispatcher instances
    let dispatcher = publisher.get_dispatcher();

    // Start a subscriber
    let mut subscriber = server_queues
        .take_ownership((MySubscriptions::ConcurrencyTest, SubscriberMode::PubSub))
        .unwrap();

    // Number of tasks and messages
    let num_tasks = 5;
    let messages_per_task = 10;
    let total_messages = num_tasks * messages_per_task;

    // Spawn multiple tasks that publish concurrently
    let mut handles = Vec::new();
    for task_id in 0..num_tasks {
        let task_dispatcher = dispatcher.clone();
        let handle = tokio::spawn(async move {
            for msg_id in 0..messages_per_task {
                let message = format!("Task {} - Message {}", task_id, msg_id).into_bytes();
                task_dispatcher
                    .publish(
                        Message::from(&message),
                        PublisherContext::new(
                            "req_id",
                            Some(format!("msg_{}_{}", task_id, msg_id)),
                        ),
                    )
                    .unwrap();

                // Add a small delay to make the test more realistic
                time::sleep(Duration::from_millis(5)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all publishing tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Allow time for all messages to be processed by RabbitMQ
    time::sleep(Duration::from_millis(500)).await;

    // Receive and count messages
    let mut received_count = 0;
    let mut unique_messages = std::collections::HashSet::new();

    while let Some(consumer_msg) = subscriber.receive().await {
        if let Some(content) = consumer_msg.content.clone() {
            let message_str = String::from_utf8_lossy(&content).to_string();
            debug!("Received message: {}", message_str);
            unique_messages.insert(message_str);
            received_count += 1;

            // Acknowledge the message
            subscriber.ack(&consumer_msg).await.unwrap();

            // Break if we've received all messages
            if received_count >= total_messages {
                break;
            }
        }
    }

    // Verify that we received the correct number of unique messages
    assert_eq!(
        unique_messages.len(),
        total_messages,
        "Expected {} unique messages, but got {}",
        total_messages,
        unique_messages.len()
    );
}

#[test_log::test(tokio::test)]
async fn rabbitmq_publish_after_close_error_test() {
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    let builder = RabbitMQBuilder::new(&connection_string, "TEST_APP")
        .publisher(MyPublishers::Orders, PublisherMode::PubSub);

    let client = match builder.build().await {
        Ok(client) => client,
        Err(err) => panic!("Failed to build: {:?}", err),
    };

    let mut client_queues = client.get_publishers();

    // Get the publisher
    let publisher = client_queues
        .take_ownership((MyPublishers::Orders, PublisherMode::PubSub))
        .unwrap();

    // Create dispatcher before closing the publisher
    let dispatcher = publisher.get_dispatcher();

    // Close the publisher
    publisher.close().await.unwrap();

    // Test message
    let message = b"message after close".to_vec();

    // Attempt to publish using the dispatcher after the publisher was closed
    let result = dispatcher.publish(
        Message::from(&message),
        PublisherContext::new("req_id", None),
    );

    // We expect this to fail with PublishError
    match result {
        Ok(_) => panic!("Expected PublishError, but publish succeeded"),
        Err(err) => {
            match err {
                RabbitMQError::PublishError => {
                    // This is the expected error
                    debug!("Got expected PublishError");
                }
                _ => panic!("Expected PublishError, but got: {:?}", err),
            }
        }
    }
}
