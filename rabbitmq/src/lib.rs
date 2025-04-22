use amqprs::{
    Ack, BasicProperties, Cancel, Close, FieldTable, Nack, Return, ShortStr,
    callbacks::{ChannelCallback, ConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, Channel, ConsumerMessage,
        ExchangeDeclareArguments, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
};
use async_trait::async_trait;
use std::{collections::HashMap, hash::Hash, marker::PhantomData};
use tokio::{
    select,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

/// Initial stage for the RabbitMQ builder
pub struct InitStage;

/// Stage for RabbitMQ builder with only subscribers configured
pub struct OnlySubscribersStage;

/// Stage for RabbitMQ builder with only publishers configured
pub struct OnlyPublishersStage;

/// Stage for RabbitMQ builder with both subscribers and publishers configured
pub struct BothStage;

/// Empty type for placeholder in generic parameters
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Empty;

impl From<&'static str> for Empty {
    fn from(_: &'static str) -> Self {
        Empty
    }
}

impl From<Empty> for &'static str {
    fn from(_: Empty) -> Self {
        "Empty"
    }
}

/// Builder for configuring and creating RabbitMQ clients and servers
/// # Type-State Builder Pattern
///
///
/// The `RabbitMQBuilder` uses a type-state pattern with three generic parameters:
/// - `Sub`: Subscriber identifier type (default: `Empty`)
/// - `Pub`: Publisher identifier type (default: `Empty`)
/// - `Stage`: Current configuration state
///
/// ## Stages
///
/// - `InitStage`: Starting point
/// - `OnlySubscribersStage`: One or more subscribers configured
/// - `OnlyPublishersStage`: One or more publishers configured
/// - `BothStage`: Both publishers and subscribers configured
///
/// ## Key Features
///
/// - **Mode-based storage**: Subscribers and publishers are stored in HashMaps using a tuple of
///   `(identifier, mode)` as the key, which prevents overrides when configuring multiple publishers
///   or subscribers with the same name but different modes.
/// - **Type-safety**: The type system ensures correct method availability at each stage
///   and guarantees that `build()` is only called after at least one
///   publisher or subscriber is configured.

#[derive(Debug)]
pub struct RabbitMQBuilder<Sub: Eq + Hash = Empty, Pub: Eq + Hash = Empty, Stage = InitStage> {
    connection_string: String,
    app_id: String,
    subscribers: Option<HashMap<(Sub, SubscriberMode), SubscriberData>>,
    publishers: Option<HashMap<(Pub, PublisherMode), PublisherData>>,
    _stage: PhantomData<Stage>,
}

/// Modes for subscriber configuration
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SubscriberMode {
    /// Simple queue subscriber (workers)
    PubSub,
    /// Fanout exchange subscriber (receives all messages)
    Broadcast,
    /// Topic exchange subscriber with specific routing patterns
    ///
    /// When using Topics mode, you specify the routing patterns (topics) at build time.
    /// The subscriber will receive all messages that match any of the specified patterns.
    /// Topics follow the AMQP topic exchange routing pattern format:
    /// - "*" (star) can substitute for exactly one word
    /// - "#"" (hash) can substitute for zero or more words
    ///
    /// Use `SubscriberMode::topics(vec!["pattern1".to_string(), "pattern2".to_string()])` for
    /// multiple patterns or `SubscriberMode::topic("pattern")` for a single pattern.
    Topics { topics: Vec<String> },
}

impl SubscriberMode {
    pub fn topics(topics: Vec<String>) -> Self {
        Self::Topics { topics }
    }

    pub fn topic(topic: &str) -> Self {
        Self::Topics {
            topics: vec![topic.to_owned()],
        }
    }

    pub fn sub() -> Self {
        Self::PubSub
    }

    pub fn worker() -> Self {
        Self::PubSub
    }

    pub fn pubsub() -> Self {
        Self::PubSub
    }

    pub fn broadcast() -> Self {
        Self::Broadcast
    }
}

/// Modes for publisher configuration
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum PublisherMode {
    /// Simple queue publisher (workers)
    PubSub,
    /// Fanout exchange publisher (broadcasts to all consumers)
    Broadcast,
    /// Topic exchange publisher with routing key support
    ///
    /// When using Topic mode, you don't need to specify the routing key (topic) at build time.
    /// Instead, provide the topic when publishing messages using `message.with_topic()` or
    /// `Message::new(content, Some(topic))`.
    Topic,
}

impl PublisherMode {
    pub fn topic() -> Self {
        Self::Topic
    }

    pub fn sub() -> Self {
        Self::PubSub
    }

    pub fn worker() -> Self {
        Self::PubSub
    }

    pub fn pubsub() -> Self {
        Self::PubSub
    }

    pub fn broadcast() -> Self {
        Self::Broadcast
    }
}

#[derive(Debug)]
pub enum SubscriberData {
    PubSub(QueueDeclareArguments, BasicConsumeArguments),
    Broadcast(
        ExchangeDeclareArguments,
        QueueDeclareArguments,
        QueueBindArguments,
        BasicConsumeArguments,
    ),
    Topic(
        ExchangeDeclareArguments,
        QueueDeclareArguments,
        Vec<QueueBindArguments>,
        BasicConsumeArguments,
    ),
}

#[derive(Debug)]
pub enum PublisherData {
    PubSub(
        QueueDeclareArguments,
        BasicPublishArguments,
        BasicProperties,
    ),
    Broadcast(
        ExchangeDeclareArguments,
        BasicPublishArguments,
        BasicProperties,
    ),
    Topic(
        ExchangeDeclareArguments,
        BasicPublishArguments,
        BasicProperties,
    ),
}

impl RabbitMQBuilder<Empty, Empty, InitStage> {
    /// Creates a new RabbitMQ builder
    ///
    /// # Arguments
    /// * `conn_str` - RabbitMQ connection string (e.g., "amqp://guest:guest@localhost:5672")
    /// * `app_id` - Application identifier used in message properties
    pub fn new(conn_str: &str, app_id: &str) -> Self {
        Self {
            connection_string: conn_str.to_owned(),
            app_id: app_id.to_owned(),
            subscribers: None,
            publishers: None,
            _stage: PhantomData,
        }
    }

    /// Adds a publisher to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Publishing mode (PubSub, Broadcast, or Topic)
    pub fn publisher<Pub: Into<&'static str> + From<&'static str> + Eq + Hash>(
        self,
        queue: Pub,
        mode: PublisherMode,
    ) -> RabbitMQBuilder<Empty, Pub, OnlyPublishersStage> {
        let mut publishers = HashMap::<(Pub, PublisherMode), PublisherData>::new();

        RabbitMQBuilder::<Empty, Pub, OnlyPublishersStage>::add_publisher(
            &mut publishers,
            queue,
            mode,
            &self.app_id,
        );

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: self.subscribers,
            publishers: Some(publishers),
            _stage: PhantomData,
        }
    }

    /// Adds a subscriber to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Subscription mode (PubSub, Broadcast, or Topics)
    pub fn subscriber<Sub: Into<&'static str> + From<&'static str> + Eq + Hash>(
        self,
        queue: Sub,
        mode: SubscriberMode,
    ) -> RabbitMQBuilder<Sub, Empty, OnlySubscribersStage> {
        let mut subscribers = HashMap::<(Sub, SubscriberMode), SubscriberData>::new();

        RabbitMQBuilder::<Sub, Empty, OnlySubscribersStage>::add_subscriber(
            queue,
            mode,
            &mut subscribers,
        );

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: Some(subscribers),
            publishers: self.publishers,
            _stage: PhantomData,
        }
    }
}

impl<Sub, Pub, Stage> RabbitMQBuilder<Sub, Pub, Stage>
where
    Sub: Into<&'static str> + From<&'static str> + Eq + Hash,
    Pub: Into<&'static str> + From<&'static str> + Eq + Hash,
{
    fn add_publisher(
        current_publsihers: &mut HashMap<(Pub, PublisherMode), PublisherData>,
        queue: Pub,
        mode: PublisherMode,
        app_id: &str,
    ) {
        let queue_str = queue.into();

        match mode {
            PublisherMode::PubSub => {
                let publish_args = BasicPublishArguments::new("", queue_str);
                let declare_args = QueueDeclareArguments::durable_client_named(queue_str);
                let basic_msg_props = BasicProperties::default()
                    .with_app_id(app_id)
                    .with_delivery_mode(2)
                    .finish();
                current_publsihers.insert(
                    (queue_str.into(), mode),
                    PublisherData::PubSub(declare_args, publish_args, basic_msg_props),
                );
            }
            PublisherMode::Broadcast => {
                let ex_declare_args = ExchangeDeclareArguments::new(queue_str, "fanout")
                    .durable(true)
                    .finish();
                let publish_args = BasicPublishArguments::new(queue_str, "");
                let basic_msg_props = BasicProperties::default()
                    .with_app_id(app_id)
                    .with_delivery_mode(1)
                    .finish();
                current_publsihers.insert(
                    (queue_str.into(), mode),
                    PublisherData::Broadcast(ex_declare_args, publish_args, basic_msg_props),
                );
            }
            PublisherMode::Topic => {
                let ex_declare_args = ExchangeDeclareArguments::new(queue_str, "topic")
                    .durable(true)
                    .finish();

                // Routing key (topic) will be provided by the user at publish time
                // via Message.with_topic() or Message::new(content, Some(topic))
                let publish_args = BasicPublishArguments::new(queue_str, "");

                let basic_msg_props = BasicProperties::default()
                    .with_app_id(app_id)
                    .with_delivery_mode(2)
                    .finish();
                current_publsihers.insert(
                    (queue_str.into(), mode),
                    PublisherData::Topic(ex_declare_args, publish_args, basic_msg_props),
                );
            }
        }
    }

    fn add_subscriber(
        queue: Sub,
        mode: SubscriberMode,
        current_subscribers: &mut HashMap<(Sub, SubscriberMode), SubscriberData>,
    ) {
        let queue_str = queue.into();

        match mode {
            SubscriberMode::PubSub => {
                let declare_args = QueueDeclareArguments::durable_client_named(queue_str);
                let consume_args = BasicConsumeArguments::new(queue_str, "");

                current_subscribers.insert(
                    (queue_str.into(), mode),
                    SubscriberData::PubSub(declare_args, consume_args),
                );
            }
            SubscriberMode::Broadcast => {
                let ex_declare_args = ExchangeDeclareArguments::new(queue_str, "fanout")
                    .durable(true)
                    .finish();
                let declare_queue_args = QueueDeclareArguments::exclusive_server_named();
                let queue_bind_args = QueueBindArguments::default()
                    .exchange(queue_str.to_owned())
                    .finish();
                let consume_args = BasicConsumeArguments::default().auto_ack(true).finish();

                current_subscribers.insert(
                    (queue_str.into(), mode),
                    SubscriberData::Broadcast(
                        ex_declare_args,
                        declare_queue_args,
                        queue_bind_args,
                        consume_args,
                    ),
                );
            }
            SubscriberMode::Topics { ref topics } => {
                let ex_declare_args = ExchangeDeclareArguments::new(queue_str, "topic")
                    .durable(true)
                    .finish();
                let declare_queue_args = QueueDeclareArguments::exclusive_server_named();

                let queue_bind_args = topics
                    .iter()
                    .map(|topic| {
                        QueueBindArguments::default()
                            .exchange(queue_str.to_owned())
                            .routing_key(topic.to_owned())
                            .finish()
                    })
                    .collect();

                let consume_args = BasicConsumeArguments::default();

                current_subscribers.insert(
                    (queue_str.into(), mode),
                    SubscriberData::Topic(
                        ex_declare_args,
                        declare_queue_args,
                        queue_bind_args,
                        consume_args,
                    ),
                );
            }
        }
    }

    async fn build_server(
        conn: &Connection,
        subs: HashMap<(Sub, SubscriberMode), SubscriberData>,
    ) -> Result<RabbitMQServer<Sub>, RabbitMQError> {
        let mut rabbit_server = RabbitMQServer::new(conn);
        for ((queue, mode), data) in subs {
            match data {
                SubscriberData::PubSub(queue_declare, consume_args) => {
                    rabbit_server
                        .set_subscription_to_queue(queue, mode, (queue_declare, consume_args))
                        .await?;
                }
                SubscriberData::Broadcast(
                    exchange_declare,
                    queue_declare_args,
                    queue_bind_args,
                    consume_args,
                ) => {
                    rabbit_server
                        .set_subscription_to_exchange(
                            queue,
                            mode,
                            (
                                exchange_declare,
                                queue_declare_args,
                                vec![queue_bind_args],
                                consume_args,
                            ),
                            true,
                        )
                        .await?;
                }
                SubscriberData::Topic(
                    exchange_declare,
                    queue_declare_args,
                    queue_bind_args,
                    consume_args,
                ) => {
                    rabbit_server
                        .set_subscription_to_exchange(
                            queue,
                            mode,
                            (
                                exchange_declare,
                                queue_declare_args,
                                queue_bind_args,
                                consume_args,
                            ),
                            false,
                        )
                        .await?;
                }
            }
        }
        Ok(rabbit_server)
    }

    async fn build_client(
        conn: &Connection,
        pubs: HashMap<(Pub, PublisherMode), PublisherData>,
    ) -> Result<RabbitMQClient<Pub>, RabbitMQError> {
        let mut client = RabbitMQClient::new(conn);

        for ((queue, mode), data) in pubs {
            match data {
                PublisherData::PubSub(queue_declare_args, basic_pub_args, basic_msg_props) => {
                    client
                        .set_publisher_to_queue(
                            queue,
                            mode,
                            (queue_declare_args, basic_pub_args, basic_msg_props),
                        )
                        .await?;
                }
                PublisherData::Broadcast(
                    exchange_declare_args,
                    basic_pub_args,
                    basic_msg_props,
                ) => {
                    client
                        .set_publisher_to_exchange(
                            queue,
                            mode,
                            (exchange_declare_args, basic_pub_args, basic_msg_props),
                        )
                        .await?;
                }
                PublisherData::Topic(exchange_declare_args, basic_pub_args, basic_msg_props) => {
                    client
                        .set_publisher_to_exchange(
                            queue,
                            mode,
                            (exchange_declare_args, basic_pub_args, basic_msg_props),
                        )
                        .await?;
                }
            }
        }

        Ok(client)
    }
}

impl<Pub> RabbitMQBuilder<Empty, Pub, OnlyPublishersStage>
where
    Pub: Into<&'static str> + From<&'static str> + Eq + Hash,
{
    /// Adds another publisher to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Publishing mode (PubSub, Broadcast, or Topic)
    ///
    /// # Returns
    /// Updated builder with the new publisher added
    pub fn publisher(
        mut self,
        queue: Pub,
        mode: PublisherMode,
    ) -> RabbitMQBuilder<Empty, Pub, OnlyPublishersStage> {
        if let Some(pubs) = self.publishers.as_mut() {
            Self::add_publisher(pubs, queue, mode, &self.app_id);
        }

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: self.subscribers,
            publishers: self.publishers,
            _stage: PhantomData,
        }
    }

    /// Adds a subscriber to the builder with existing publishers
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Subscription mode (PubSub, Broadcast, or Topics)
    pub fn subscriber<Sub: Into<&'static str> + From<&'static str> + Eq + Hash>(
        self,
        queue: Sub,
        mode: SubscriberMode,
    ) -> RabbitMQBuilder<Sub, Pub, BothStage> {
        let mut subscribers = HashMap::<(Sub, SubscriberMode), SubscriberData>::new();

        RabbitMQBuilder::<Sub, Pub, BothStage>::add_subscriber(queue, mode, &mut subscribers);

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: Some(subscribers),
            publishers: self.publishers,
            _stage: PhantomData,
        }
    }

    /// Builds and returns a RabbitMQ client with configured publishers
    ///
    /// # Returns
    /// A RabbitMQ client ready to publish messages
    ///
    /// # Errors
    /// Returns an error if establishing connection or setting up publishers fails
    pub async fn build(self) -> Result<RabbitMQClient<Pub>, RabbitMQError> {
        tracing::info!("Building RabbitMQ client with publishers only");
        tracing::info!("Connecting to RabbitMQ at: {}", self.connection_string);
        
        let conn = open_rabbit_connection(&self.connection_string).await?;

        tracing::info!("Setting up publishers");
        let pubs = Self::build_client(&conn, self.publishers.unwrap()).await?;
        tracing::info!("RabbitMQ client build successful");

        Ok(pubs)
    }
}

impl<Sub> RabbitMQBuilder<Sub, Empty, OnlySubscribersStage>
where
    Sub: Into<&'static str> + From<&'static str> + Eq + Hash,
{
    /// Adds a publisher to the builder with existing subscribers
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Publishing mode (PubSub, Broadcast, or Topic)
    ///
    /// # Returns
    /// Updated builder with both subscribers and publishers configured
    pub fn publisher<Pub: Into<&'static str> + From<&'static str> + Eq + Hash>(
        self,
        queue: Pub,
        mode: PublisherMode,
    ) -> RabbitMQBuilder<Sub, Pub, BothStage> {
        let mut publishers = HashMap::<(Pub, PublisherMode), PublisherData>::new();

        RabbitMQBuilder::<Sub, Pub, BothStage>::add_publisher(
            &mut publishers,
            queue,
            mode,
            &self.app_id,
        );

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: self.subscribers,
            publishers: Some(publishers),
            _stage: PhantomData,
        }
    }

    /// Adds another subscriber to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Subscription mode (PubSub, Broadcast, or Topics)
    ///
    /// # Returns
    /// Updated builder with the new subscriber added
    pub fn subscriber(
        mut self,
        queue: Sub,
        mode: SubscriberMode,
    ) -> RabbitMQBuilder<Sub, Empty, OnlySubscribersStage> {
        if let Some(subs) = self.subscribers.as_mut() {
            Self::add_subscriber(queue, mode, subs);
        }

        RabbitMQBuilder {
            connection_string: self.connection_string,
            app_id: self.app_id,
            subscribers: self.subscribers,
            publishers: self.publishers,
            _stage: PhantomData,
        }
    }

    /// Builds and returns a RabbitMQ server with configured subscribers
    ///
    /// # Returns
    /// A RabbitMQ server ready to receive messages
    ///
    /// # Errors
    /// Returns an error if establishing connection or setting up subscriptions fails
    pub async fn build(self) -> Result<RabbitMQServer<Sub>, RabbitMQError> {
        tracing::info!("Building RabbitMQ server with subscribers only");
        tracing::info!("Connecting to RabbitMQ at: {}", self.connection_string);
        
        let conn = open_rabbit_connection(&self.connection_string).await?;

        tracing::info!("Setting up subscribers");
        let subs = Self::build_server(&conn, self.subscribers.unwrap()).await?;
        tracing::info!("RabbitMQ server build successful");

        Ok(subs)
    }
}

impl<Sub, Pub> RabbitMQBuilder<Sub, Pub, BothStage>
where
    Sub: Into<&'static str> + From<&'static str> + Eq + Hash,
    Pub: Into<&'static str> + From<&'static str> + Eq + Hash,
{
    /// Adds another publisher to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Publishing mode (PubSub, Broadcast, or Topic)
    ///
    /// # Returns
    /// Updated builder with the new publisher added
    pub fn publisher(
        mut self,
        queue: Pub,
        mode: PublisherMode,
    ) -> RabbitMQBuilder<Sub, Pub, BothStage> {
        if let Some(pubs) = self.publishers.as_mut() {
            Self::add_publisher(pubs, queue, mode, &self.app_id);
        }

        self
    }

    /// Adds another subscriber to the builder
    ///
    /// # Arguments
    /// * `queue` - Queue or exchange name
    /// * `mode` - Subscription mode (PubSub, Broadcast, or Topics)
    ///
    /// # Returns
    /// Updated builder with the new subscriber added
    pub fn subscriber(
        mut self,
        queue: Sub,
        mode: SubscriberMode,
    ) -> RabbitMQBuilder<Sub, Pub, BothStage> {
        if let Some(subs) = self.subscribers.as_mut() {
            Self::add_subscriber(queue, mode, subs);
        }

        self
    }

    /// Builds and returns both a RabbitMQ client and server
    ///
    /// # Returns
    /// A tuple containing a RabbitMQ client and server
    ///
    /// # Errors
    /// Returns an error if establishing connection or setting up publishers/subscribers fails
    pub async fn build(self) -> Result<(RabbitMQClient<Pub>, RabbitMQServer<Sub>), RabbitMQError> {
        tracing::info!("Building RabbitMQ client and server with both publishers and subscribers");
        tracing::info!("Connecting to RabbitMQ at: {}", self.connection_string);
        
        let conn = open_rabbit_connection(&self.connection_string).await?;

        tracing::info!("Setting up publishers");
        let pubs = Self::build_client(&conn, self.publishers.unwrap()).await?;
        
        tracing::info!("Setting up subscribers");
        let subs = Self::build_server(&conn, self.subscribers.unwrap()).await?;
        
        tracing::info!("RabbitMQ client and server build successful");
        Ok((pubs, subs))
    }
}

/// Client for publishing messages to RabbitMQ
pub struct RabbitMQClient<P: Into<&'static str> + From<&'static str> + Eq + Hash> {
    conn: Connection,
    publishers: RabbitMQRegistry<(P, PublisherMode), Publisher>,
}

impl<P: Into<&'static str> + From<&'static str> + Eq + Hash> RabbitMQClient<P> {
    /// Creates a new RabbitMQ client
    ///
    /// # Arguments
    /// * `conn` - RabbitMQ connection
    fn new(conn: &Connection) -> Self {
        Self {
            conn: conn.clone(),
            publishers: RabbitMQRegistry::new(),
        }
    }

    /// Sets up a publisher for a queue
    ///
    /// # Arguments
    /// * `queue` - Queue identifier
    /// * `queue_declare` - Queue declaration arguments
    /// * `publish_args` - Basic publish arguments
    /// * `basic_props` - Basic message properties
    ///
    /// # Errors
    /// Returns an error if channel opening, queue declaration, or publisher setup fails
    async fn set_publisher_to_queue(
        &mut self,
        queue: P,
        mode: PublisherMode,
        (queue_declare, publish_args, basic_props): (
            QueueDeclareArguments,
            BasicPublishArguments,
            BasicProperties,
        ),
    ) -> Result<(), RabbitMQError> {
        let channel = open_rabbit_channel(&self.conn)
            .await
            .map_err(|err| RabbitMQError::OpenChannelError(err.to_string()))?;

        let _ = channel
            .queue_declare(queue_declare)
            .await
            .map_err(|err| RabbitMQError::QueueDeclarationError(err.to_string()))?;

        let queue_str: &str = queue.into();

        let queue_publisher = Publisher::new(
            queue_str,
            mode,
            publish_args,
            basic_props,
            self.conn.clone(),
            channel,
        );

        self.publishers
            .insert((queue_str.into(), mode), queue_publisher);

        Ok(())
    }

    /// Sets up a publisher for an exchange
    ///
    /// # Arguments
    /// * `queue` - Exchange identifier
    /// * `exchange_declare` - Exchange declaration arguments
    /// * `publish_args` - Basic publish arguments
    /// * `basic_props` - Basic message properties
    ///
    /// # Errors
    /// Returns an error if channel opening, exchange declaration, or publisher setup fails
    async fn set_publisher_to_exchange(
        &mut self,
        queue: P,
        mode: PublisherMode,
        (exchange_declare, publish_args, basic_props): (
            ExchangeDeclareArguments,
            BasicPublishArguments,
            BasicProperties,
        ),
    ) -> Result<(), RabbitMQError> {
        let channel = open_rabbit_channel(&self.conn)
            .await
            .map_err(|err| RabbitMQError::OpenChannelError(err.to_string()))?;

        let queue_str: &str = queue.into();

        channel
            .exchange_declare(exchange_declare)
            .await
            .map_err(|err| RabbitMQError::ExchangeDeclarationError(err.to_string()))?;

        let queue_publisher = Publisher::new(
            queue_str,
            mode,
            publish_args,
            basic_props,
            self.conn.clone(),
            channel,
        );

        self.publishers
            .insert((queue_str.into(), mode), queue_publisher);

        Ok(())
    }

    /// Returns all configured publishers
    ///
    /// # Returns
    /// Map of publishers identified by their queue/exchange names
    pub fn get_publishers(self) -> RabbitMQRegistry<(P, PublisherMode), Publisher> {
        self.publishers
    }
}

/// Server for consuming messages from RabbitMQ
pub struct RabbitMQServer<S: Into<&'static str> + From<&'static str> + Eq + Hash> {
    conn: Connection,
    subscribers: RabbitMQRegistry<(S, SubscriberMode), Subscription>,
}

impl<S: Into<&'static str> + From<&'static str> + Eq + Hash> RabbitMQServer<S> {
    /// Creates a new RabbitMQ server
    ///
    /// # Arguments
    /// * `conn` - RabbitMQ connection
    fn new(conn: &Connection) -> Self {
        Self {
            conn: conn.clone(),
            subscribers: RabbitMQRegistry::new(),
        }
    }

    /// Sets up a subscription to a queue
    ///
    /// # Arguments
    /// * `queue` - Queue identifier
    /// * `subscriber_args` - Queue declaration arguments
    /// * `consume_args` - Consumption parameters
    ///
    /// # Errors
    /// Returns an error if channel opening, queue declaration, or subscription setup fails
    async fn set_subscription_to_queue(
        &mut self,
        queue: S,
        mode: SubscriberMode,
        (subscriber_args, consume_args): (QueueDeclareArguments, BasicConsumeArguments),
    ) -> Result<(), RabbitMQError> {
        let channel = open_rabbit_channel(&self.conn).await?;

        let _ = channel
            .queue_declare(subscriber_args)
            .await
            .map_err(|err| RabbitMQError::QueueDeclarationError(err.to_string()))?;

        // TODO: debug log stuff here

        let (_ctag, rx) = channel
            .basic_consume_rx(consume_args)
            .await
            .map_err(|err| RabbitMQError::SubscriptionError(err.to_string()))?;

        let queue_str: &str = queue.into();
        self.subscribers.insert(
            (queue_str.into(), mode),
            Subscription::new(queue_str, queue_str, rx, self.conn.clone(), channel, false),
        );

        Ok(())
    }

    /// Sets up a subscription to an exchange
    ///
    /// # Arguments
    /// * `queue` - Exchange identifier
    /// * `exchange_args` - Exchange declaration arguments
    /// * `queue_declare_args` - Queue declaration arguments
    /// * `queue_bind_args` - Queue binding arguments
    /// * `consume_args` - Consumption parameters
    /// * `auto_ack` - Whether messages should be auto-acknowledged
    ///
    /// # Errors
    /// Returns an error if channel opening, exchange/queue declaration, binding, or subscription setup fails
    async fn set_subscription_to_exchange(
        &mut self,
        queue: S,
        mode: SubscriberMode,
        (exchange_args, queue_declare_args, queue_bind_args, mut consume_args): (
            ExchangeDeclareArguments,
            QueueDeclareArguments,
            Vec<QueueBindArguments>,
            BasicConsumeArguments,
        ),
        auto_ack: bool,
    ) -> Result<(), RabbitMQError> {
        let channel = open_rabbit_channel(&self.conn).await?;

        channel
            .exchange_declare(exchange_args)
            .await
            .map_err(|err| RabbitMQError::ExchangeDeclarationError(err.to_string()))?;

        let (queue_name, _, _) = channel
            .queue_declare(queue_declare_args)
            .await
            .map_err(|err| RabbitMQError::QueueDeclarationError(err.to_string()))?
            .unwrap(); // it's safe to do unwrap since no_wait is false

        let queue_str: &str = queue.into();

        for mut args in queue_bind_args {
            let q = args.queue(queue_name.clone()).finish();
            channel
                .queue_bind(q)
                .await
                .map_err(|err| RabbitMQError::QueueBindingError(err.to_string()))?;
        }

        let (_ctag, rx) = channel
            .basic_consume_rx(consume_args.queue(queue_name.clone()).finish())
            .await
            .map_err(|err| RabbitMQError::SubscriptionError(err.to_string()))?;

        self.subscribers.insert(
            (queue_str.into(), mode),
            Subscription::new(
                queue_str,
                &queue_name,
                rx,
                self.conn.clone(),
                channel,
                auto_ack,
            ),
        );

        Ok(())
    }

    /// Returns all configured subscribers
    ///
    /// # Returns
    /// Map of subscribers identified by their queue/exchange names
    pub fn get_subscribers(self) -> RabbitMQRegistry<(S, SubscriberMode), Subscription> {
        self.subscribers
    }
}

/// Container for RabbitMQ publishers or subscribers
///
/// Generic container that manages a collection of publishers or subscribers
/// identified by keys of type T. This provides abstraction over the actual
/// storage mechanism and allows for safe ownership transfer of individual items.
///
/// # Type Parameters
/// * `T` - Type used as keys (must implement Eq + Hash)
/// * `U` - Type of values stored (publishers or subscribers)
pub struct RabbitMQRegistry<T: Eq + Hash, U>(HashMap<T, U>);

impl<T: Eq + Hash, U> RabbitMQRegistry<T, U> {
    fn new() -> Self {
        Self(HashMap::default())
    }

    fn insert(&mut self, q: T, qq: U) {
        self.0.insert(q, qq);
    }

    /// Takes ownership of a specific queue
    ///
    /// # Arguments
    /// * `q` - Queue/exchange identifier
    ///
    /// # Returns
    /// The publisher or subscriber associated with the given identifier
    ///
    /// # Errors
    /// Returns an error if the queue doesn't exist
    pub fn take_ownership(&mut self, q: T) -> Result<U, RabbitMQError> {
        self.0.remove(&q).ok_or(RabbitMQError::NotAQueue)
    }
}

/// Subscription for consuming messages from a RabbitMQ queue or exchange
///
/// ## Architecture
///
/// The Subscription operates using a pull-based model for message consumption:
///
/// 1. **Message Channel**: The subscription is backed by an UnboundedReceiver that receives
///    messages from RabbitMQ as they arrive
/// 2. **Pull-based API**: Messages are retrieved via the `receive()` method, which allows
///    for controlled consumption based on application need
/// 3. **Acknowledgment System**: Messages can be explicitly acknowledged after processing
///    (unless auto-acknowledgment is enabled)
/// 4. **Direct Channel Access**: Unlike Publisher, Subscription directly uses the RabbitMQ
///    channel without a background task
///
/// ## Message Flow
///
/// When consuming messages:
/// 1. Call `receive()` to get the next available message
/// 2. Process the message according to your application logic
/// 3. Call `ack()` to acknowledge the message (if auto-ack is not enabled)
/// 4. Repeat as needed for more messages
///
/// ## Cleanup
///
/// IMPORTANT: The `close()` method MUST be called to ensure a graceful shutdown.
/// Simply dropping the Subscription will not guarantee proper cleanup.
///
/// When `close()` is called:
/// 1. The RabbitMQ channel is properly closed
/// 2. Resources associated with the subscription are released
///
/// Without calling `close()`, the subscription's resources may not be properly
/// cleaned up, potentially leading to resource leaks or connection issues.
pub struct Subscription {
    queue_name: String,
    rabbit_queue_name: String,
    consumer: UnboundedReceiver<ConsumerMessage>,
    rabbit_connection: (Channel, Connection),
    auto_ack_flag: bool,
}

impl Subscription {
    /// Creates a new Subscription instance
    ///
    /// This method sets up a subscription to a RabbitMQ queue or exchange.
    /// The subscription maintains:
    /// - A logical queue name for application use
    /// - The actual RabbitMQ queue name (may differ for some exchange types)
    /// - A consumer channel for receiving messages
    /// - Connection and channel references for RabbitMQ operations
    /// - Auto-acknowledgment flag that determines acknowledgment behavior
    ///
    /// # Arguments
    /// * `queue_name` - Logical name of the queue or exchange
    /// * `rabbit_queue_name` - Actual RabbitMQ queue name
    /// * `consumer` - Channel for receiving messages
    /// * `rabbit_connection` - RabbitMQ connection
    /// * `rabbit_connection_channel` - RabbitMQ channel for consuming
    /// * `auto_ack` - Whether messages should be auto-acknowledged
    fn new(
        queue_name: &str,
        rabbit_queue_name: &str,
        consumer: UnboundedReceiver<ConsumerMessage>,
        rabbit_connection: Connection,
        rabbit_connection_channel: Channel,
        auto_ack: bool,
    ) -> Self {
        Self {
            queue_name: queue_name.to_owned(),
            rabbit_queue_name: rabbit_queue_name.to_owned(),
            consumer,
            rabbit_connection: (rabbit_connection_channel, rabbit_connection),
            auto_ack_flag: auto_ack,
        }
    }

    /// Returns the queue name
    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    /// Returns the RabbitMQ queue name (may differ from the logical queue name)
    pub fn rabbit_queue_name(&self) -> &str {
        &self.rabbit_queue_name
    }

    /// Receives the next message from the subscription
    ///
    /// # Returns
    /// The next message or None if the channel is closed
    pub async fn receive(&mut self) -> Option<ConsumerMessage> {
        self.consumer.recv().await
    }

    /// Acknowledges a message as processed
    ///
    /// If auto-acknowledgment is enabled, this method does nothing.
    /// Otherwise, it sends an acknowledgment to RabbitMQ for the given message.
    ///
    /// # Arguments
    /// * `message` - The message to acknowledge
    ///
    /// # Returns
    /// Ok(()) on success or an error if acknowledgment fails
    ///
    /// # Errors
    /// Returns an error if the message lacks delivery information or if the acknowledgment fails
    pub async fn ack(&self, message: &ConsumerMessage) -> Result<(), RabbitMQError> {
        if self.auto_ack_flag {
            debug!("ack not needed");
            return Ok(());
        }

        if let Some(deliver_info) = &message.deliver {
            let ack_args = BasicAckArguments::new(deliver_info.delivery_tag(), false);
            self.rabbit_connection
                .0
                .basic_ack(ack_args)
                .await
                .map_err(|err| RabbitMQError::AckMessageError(err.to_string()))?;

            return Ok(());
        }

        Err(RabbitMQError::NotDeliveryTag)
    }

    /// Closes the subscription and its channel
    ///
    /// IMPORTANT: This method MUST be called when you're done with the subscription
    /// to ensure proper cleanup of resources. Simply dropping the Subscription
    /// will not properly clean up the RabbitMQ connection.
    ///
    /// This method:
    /// 1. Properly closes the RabbitMQ channel
    /// 2. Releases any resources associated with the subscription
    /// 3. Ensures no more messages will be received
    ///
    /// After calling this method, the subscription cannot be used anymore.
    /// Any attempt to use it will result in an error.
    ///
    /// # Returns
    /// Ok(()) on successful cleanup
    ///
    /// # Errors
    /// Returns an error if closing the channel fails
    pub async fn close(self) -> Result<(), RabbitMQError> {
        self.rabbit_connection
            .1
            .close()
            .await
            .map_err(|err| RabbitMQError::CloseChannelError(err.to_string()))?;

        Ok(())
    }
}

/// Internal message type used for asynchronous communication between publishers and the RabbitMQ channel
///
/// This struct encapsulates all the information needed to publish a message to RabbitMQ:
/// - The message content as bytes
/// - AMQP message properties (headers, delivery mode, etc.)  
/// - Publishing arguments (exchange, routing key, etc.)
///
/// It's used internally by Publisher and PublisherDispatcher to send messages to
/// the channel handling task via an UnboundedSender.
struct RabbitPublishMessage(Vec<u8>, BasicProperties, BasicPublishArguments);

/// Publisher for sending messages to a RabbitMQ queue or exchange
///
/// ## Architecture
///
/// The Publisher operates using a background task model for non-blocking message delivery:
///
/// 1. **Message Channel**: When created, the Publisher sets up an mpsc channel for message passing
/// 2. **Background Task**: A tokio task is spawned that continuously listens for messages to publish
/// 3. **Cancellation Token**: A CancellationToken is used to gracefully shut down the background task
/// 4. **Non-blocking API**: The publish() method is non-blocking, sending messages to the background task
///
/// ## Message Flow
///
/// When `publish()` is called:
/// 1. The message is wrapped in a `RabbitPublishMessage` with necessary metadata
/// 2. It's sent through the mpsc channel to the background task
/// 3. The background task receives the message and publishes it to RabbitMQ
/// 4. Any errors are logged but don't block the caller
///
/// ## Cleanup
///
/// IMPORTANT: The `close()` method MUST be called to ensure a graceful shutdown.
/// Simply dropping the Publisher will not guarantee proper cleanup.
///
/// When `close()` is called:
/// 1. The cancellation token is triggered
/// 2. The background task detects this and shuts down gracefully
/// 3. The RabbitMQ channel is closed properly
///
/// Without calling `close()`, the background task may continue running even after
/// the Publisher is dropped, which can lead to resource leaks or unexpected behavior.
///
pub struct Publisher {
    queue_name: String,
    mode: PublisherMode,
    pub_args: BasicPublishArguments,
    msg_common_props: BasicProperties,
    rabbit_connection: (Channel, Connection),
    dispatcher: UnboundedSender<RabbitPublishMessage>,
    _handler: (JoinHandle<()>, CancellationToken),
}

impl Publisher {
    /// Creates a new Publisher instance
    ///
    /// This method sets up:
    /// 1. A communication channel for message passing
    /// 2. A background task that continuously processes messages
    /// 3. A cancellation token for graceful shutdown
    ///
    /// The background task will:
    /// - Listen for messages on the channel
    /// - Publish received messages to RabbitMQ
    /// - Watch for cancellation signals
    /// - Log any errors that occur during publishing
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue or exchange
    /// * `mode` - Publishing mode (PubSub, Broadcast, or Topic)
    /// * `pub_args` - Publishing arguments
    /// * `msg_common_props` - Common message properties to use
    /// * `rabbit_connection` - RabbitMQ connection
    /// * `rabbit_connection_channel` - RabbitMQ channel for publishing
    fn new(
        queue_name: &str,
        mode: PublisherMode,
        pub_args: BasicPublishArguments,
        msg_common_props: BasicProperties,
        rabbit_connection: Connection,
        rabbit_connection_channel: Channel,
    ) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<RabbitPublishMessage>();

        let channel = rabbit_connection_channel.clone();
        let queue = queue_name.to_owned();

        let cancel_token = CancellationToken::new();

        let cloned_token = cancel_token.clone();
        let handler = tokio::spawn(async move {
            loop {
                select! {
                    _ = cloned_token.cancelled() => {
                        debug!("publisher was closed");
                        return
                    },
                    message = rx.recv() => {
                        match message {
                            Some(msg) => {
                                if let Err(err) = channel.basic_publish(msg.1, msg.0, msg.2).await {
                                    error!("error while publishing to {}: {}", queue, err)
                                }
                            },
                            None => {
                                error!("unexpected channel close")
                            }
                        }

                    }
                }
            }
        });

        Self {
            queue_name: queue_name.to_owned(),
            mode,
            pub_args,
            msg_common_props,
            rabbit_connection: (rabbit_connection_channel, rabbit_connection),
            dispatcher: tx,
            _handler: (handler, cancel_token),
        }
    }

    /// Returns the queue name
    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    pub fn mode(&self) -> &PublisherMode {
        &self.mode
    }

    /// Publishes a message to the queue or exchange
    ///
    /// This method is non-blocking. It sends the message to the background
    /// task, which handles the actual publishing to RabbitMQ asynchronously.
    /// Errors in publishing are logged by the background task but are not
    /// returned to the caller of this method.
    ///
    /// For topic exchanges, you must provide the topic in the Message
    /// using `message.with_topic()` or `Message::new(content, Some(topic))`.
    ///
    /// # Arguments
    /// * `message` - Message content and optional topic
    /// * `ctx` - Publishing context containing metadata such as request ID and message ID
    ///
    /// # Errors
    /// Returns `RabbitMQError::MissingTopic` if publishing to a topic exchange without a topic
    /// Returns `RabbitMQError::PublishError` if the channel to the background task was closed
    pub fn publish(&self, message: Message, ctx: PublisherContext) -> Result<(), RabbitMQError> {
        let publisher_args =
            Self::build_publish_arguments(self.mode, &self.pub_args, message.topic)?;
        let message_props = ctx.into_basic_props(&self.msg_common_props);

        self.dispatcher
            .send(RabbitPublishMessage(
                message.content,
                message_props,
                publisher_args,
            ))
            .map_err(|_| RabbitMQError::PublishError)?;

        Ok(())
    }

    /// Closes the publisher and its channel
    ///
    /// This method performs a graceful shutdown:
    /// 1. Triggers the cancellation token to signal the background task to stop
    /// 2. Closes the RabbitMQ channel
    ///
    /// The background task will detect the cancellation signal and terminate,
    /// ensuring no new messages are processed after this point.
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns an error if closing the channel fails
    pub async fn close(self) -> Result<(), RabbitMQError> {
        self._handler.1.cancel();
        self.rabbit_connection
            .0
            .close()
            .await
            .map_err(|err| RabbitMQError::CloseChannelError(err.to_string()))?;

        Ok(())
    }

    /// Builds the publish arguments for the given mode and topic
    ///
    /// For Topic mode publishers, this method requires a topic to be provided
    /// in the message. If no topic is provided, it returns `RabbitMQError::MissingTopic`.
    ///
    /// # Arguments
    /// * `mode` - The publishing mode
    /// * `publisher_arguments` - Base publishing arguments to modify
    /// * `topic` - Optional topic/routing key for the message
    ///
    /// # Returns
    /// Modified publish arguments with routing key set if needed
    ///
    /// # Errors
    /// Returns `RabbitMQError::MissingTopic` if mode is Topic but no topic is provided
    fn build_publish_arguments(
        mode: PublisherMode,
        publisher_arguments: &BasicPublishArguments,
        topic: Option<String>,
    ) -> Result<BasicPublishArguments, RabbitMQError> {
        match mode {
            PublisherMode::Topic => {
                if topic.is_none() {
                    return Err(RabbitMQError::MissingTopic);
                }

                Ok(publisher_arguments
                    .clone()
                    .routing_key(topic.unwrap())
                    .finish())
            }
            _ => Ok(publisher_arguments.clone()),
        }
    }

    /// Creates and returns a PublisherDispatcher for this publisher
    ///
    /// The dispatcher allows publishing messages from multiple threads or tasks
    /// without sharing the actual Publisher instance. This is useful for concurrent
    /// publishing from multiple parts of your application.
    ///
    /// Both the Publisher and any created PublisherDispatcher instances will send
    /// messages to the same background task. This means:
    ///
    /// - You can create multiple dispatchers from a single publisher
    /// - All dispatchers share the same communication channel
    /// - When the original Publisher is closed, its background task is stopped,
    ///   affecting all dispatchers
    ///
    /// # Returns
    /// A new PublisherDispatcher instance that can be cloned and shared
    pub fn get_dispatcher(&self) -> PublisherDispatcher {
        PublisherDispatcher::new(self)
    }
}

#[derive(Debug, Clone)]
/// A lightweight clone of a Publisher that can be shared between multiple threads or async tasks
///
/// ## Features
///
/// - **Thread-safe**: Can be cloned and used from multiple async tasks or threads
/// - **Non-blocking**: Uses message passing to avoid blocking the caller thread
/// - **Same API**: Uses the same publishing API as the main Publisher
/// - **Message-based communication**: Uses a message dispatcher to communicate with the
///   RabbitMQ channel without directly owning or referencing it
/// ```
pub struct PublisherDispatcher {
    dispatcher: UnboundedSender<RabbitPublishMessage>,
    mode: PublisherMode,
    pub_args: BasicPublishArguments,
    msg_common_props: BasicProperties,
}

impl PublisherDispatcher {
    fn new(publisher: &Publisher) -> Self {
        Self {
            dispatcher: publisher.dispatcher.clone(),
            mode: publisher.mode,
            pub_args: publisher.pub_args.clone(),
            msg_common_props: publisher.msg_common_props.clone(),
        }
    }

    /// Publishes a message to the queue or exchange
    ///
    /// This method has the same signature as `Publisher::publish`, allowing it to be
    /// used interchangeably. For topic exchanges, you must provide the topic in the Message
    /// using `message.with_topic()` or `Message::new(content, Some(topic))`.
    ///
    /// # Arguments
    /// * `message` - Message content and optional topic
    /// * `ctx` - Publishing context containing metadata such as request ID and message ID
    ///
    /// # Errors
    /// Returns `RabbitMQError::MissingTopic` if publishing to a topic exchange without a topic
    /// Returns `RabbitMQError::PublishError` if the publishing channel was closed or dropped
    pub fn publish(&self, message: Message, ctx: PublisherContext) -> Result<(), RabbitMQError> {
        let publisher_args =
            Publisher::build_publish_arguments(self.mode, &self.pub_args, message.topic)?;
        let message_props = ctx.into_basic_props(&self.msg_common_props);

        self.dispatcher
            .send(RabbitPublishMessage(
                message.content,
                message_props,
                publisher_args,
            ))
            .map_err(|_| RabbitMQError::PublishError)?;

        Ok(())
    }
}

/// Message to be published to RabbitMQ
///
/// This struct holds the content of the message and an optional topic for topic exchanges.
/// When publishing to a topic exchange, the topic must be provided using `with_topic()`
/// or by creating the message with `Message::new(content, Some(topic))`.
pub struct Message {
    content: Vec<u8>,
    topic: Option<String>,
}

impl<T: AsRef<[u8]>> From<T> for Message {
    fn from(value: T) -> Self {
        Message {
            content: value.as_ref().to_vec(),
            topic: None,
        }
    }
}

impl Message {
    pub fn new(content: Vec<u8>, topic: Option<String>) -> Self {
        Self { content, topic }
    }

    pub fn content(content: Vec<u8>) -> Self {
        Self {
            content,
            topic: None,
        }
    }

    pub fn content_ref<T: AsRef<[u8]>>(content: T) -> Self {
        Self {
            content: content.as_ref().to_vec(),
            topic: None,
        }
    }

    /// Adds a routing topic to the message
    ///
    /// This is required when publishing to a topic exchange.
    ///
    /// # Arguments
    /// * `topic` - The routing key (topic) for the message
    ///
    /// # Returns
    /// The same message with the topic added
    pub fn with_topic(self, topic: &str) -> Self {
        Self {
            content: self.content,
            topic: Some(topic.to_owned()),
        }
    }
}

/// Context information for message publishing
///
/// Provides additional metadata for published messages, such as request IDs
/// and message IDs for tracing and correlation.
pub struct PublisherContext {
    request_id: String,
    message_id: Option<String>,
}

impl PublisherContext {
    /// Creates a new publisher context with request ID and optional message ID
    ///
    /// # Arguments
    /// * `req_id` - Request identifier for message tracing
    /// * `message_id` - Optional unique message identifier
    pub fn new(req_id: &str, message_id: Option<String>) -> Self {
        Self {
            request_id: req_id.to_owned(),
            message_id,
        }
    }

    /// Converts the context into AMQP message properties
    ///
    /// Adds the request ID as a header and optionally sets the message ID.
    ///
    /// # Arguments
    /// * `current_basic_props` - Base properties to extend with context information
    ///
    /// # Returns
    /// Enhanced AMQP message properties with context information
    fn into_basic_props(self, current_basic_props: &BasicProperties) -> BasicProperties {
        let mut new_basic_props = current_basic_props.clone();
        if let Some(msg_id) = self.message_id {
            new_basic_props.with_message_id(&msg_id);
        }

        let mut headers = FieldTable::new();

        headers.insert(
            // Safe to do unwrap() here - Only fails if given &str length > u8 max (256)
            ShortStr::try_from("request_id").unwrap(),
            self.request_id.into(),
        );

        new_basic_props.with_headers(headers);

        new_basic_props
    }
}

/// Error types for RabbitMQ operations
#[derive(Debug, thiserror::Error)]
pub enum RabbitMQError {
    /// Error in the provided URI
    #[error("Provided URI Error: {0}")]
    UriError(String),
    /// Error establishing connection
    #[error("Connection error: {0}")]
    ConnectionError(String),
    /// Error opening a channel
    #[error("Error while opening a rabbitmq channel: {0}")]
    OpenChannelError(String),
    /// Error declaring a queue
    #[error("Error while declaring a queue: {0}")]
    QueueDeclarationError(String),
    /// Error declaring an exchange
    #[error("Error while declaring a exchange: {0}")]
    ExchangeDeclarationError(String),
    /// Error starting to consume from a subscription
    #[error("Error while starting to consume from a subscription: {0}")]
    SubscriptionError(String),
    /// Error binding a queue to an exchange
    #[error("Error while binding a queue to exchange: {0}")]
    QueueBindingError(String),
    /// Error closing a channel
    #[error("Error while closing a channel: {0}")]
    CloseChannelError(String),
    /// Error publishing a message
    #[error("Error while publishing a message - channel was dropped or closed")]
    PublishError,
    /// Error when a queue doesn't exist
    #[error("Not registered queue")]
    NotAQueue,
    /// Error while acknowledging a message failed
    #[error("Error while acknowledging a message: {0}")]
    AckMessageError(String),
    /// Message does not contain delivery tag
    #[error("Unexpected error: message does not contain delivery tag")]
    NotDeliveryTag,
    /// Missing topic when publishing to a topic exchange
    ///
    /// This error occurs when attempting to publish a message to a topic exchange
    /// without providing a topic. Use `Message::with_topic()` or
    /// `Message::new(content, Some(topic))` to add a topic to your message.
    #[error("Topic mode Publisher MUST have a topic")]
    MissingTopic,
}

async fn open_rabbit_connection(connection_string: &str) -> Result<Connection, RabbitMQError> {
    tracing::info!("Attempting to open RabbitMQ connection to: {}", connection_string);
    
    let open_conn_args = match OpenConnectionArguments::try_from(connection_string) {
        Ok(args) => {
            tracing::info!("Successfully parsed connection arguments");
            args
        },
        Err(err) => {
            tracing::error!("Failed to parse connection string: {}", err);
            return Err(RabbitMQError::UriError(err.to_string()));
        }
    };

    tracing::info!("Connecting to RabbitMQ server...");
    let conn = match Connection::open(&open_conn_args).await {
        Ok(conn) => {
            tracing::info!("Successfully established connection to RabbitMQ");
            conn
        },
        Err(err) => {
            tracing::error!("Failed to connect to RabbitMQ: {}", err);
            return Err(RabbitMQError::ConnectionError(err.to_string()));
        }
    };

    tracing::info!("Registering connection callback");
    match conn.register_callback(RabbitConnectionCallback).await {
        Ok(_) => tracing::info!("Successfully registered connection callback"),
        Err(err) => {
            tracing::error!("Failed to register connection callback: {}", err);
            return Err(RabbitMQError::ConnectionError(err.to_string()));
        }
    }

    tracing::info!("RabbitMQ connection established successfully");
    Ok(conn)
}

async fn open_rabbit_channel(conn: &Connection) -> Result<Channel, RabbitMQError> {
    tracing::info!("Opening RabbitMQ channel");
    
    let rabbit_channel = match conn.open_channel(None).await {
        Ok(ch) => {
            tracing::info!("Successfully opened channel");
            ch
        },
        Err(err) => {
            tracing::error!("Failed to open channel: {}", err);
            return Err(RabbitMQError::OpenChannelError(err.to_string()));
        }
    };

    tracing::info!("Registering channel callback");
    match rabbit_channel.register_callback(RabbitChannelCallback).await {
        Ok(_) => tracing::info!("Successfully registered channel callback"),
        Err(err) => {
            tracing::error!("Failed to register channel callback: {}", err);
            return Err(RabbitMQError::OpenChannelError(err.to_string()));
        }
    }

    tracing::info!("RabbitMQ channel opened successfully");
    Ok(rabbit_channel)
}

struct RabbitConnectionCallback;

#[async_trait]
impl ConnectionCallback for RabbitConnectionCallback {
    async fn close(
        &mut self,
        _connection: &Connection,
        close: Close,
    ) -> Result<(), amqprs::error::Error> {
        debug!("connection closed {:?}", close);
        Ok(())
    }

    /// Callback to handle connection `blocked` indication from server
    async fn blocked(&mut self, _connection: &Connection, reason: String) {
        debug!("connection blocked {:?}", reason);
    }
    /// Callback to handle connection `unblocked` indication from server
    async fn unblocked(&mut self, _connection: &Connection) {
        debug!("connection unblocked ");
    }

    /// Callback to handle secret updated indication from server
    async fn secret_updated(&mut self, _connection: &Connection) {
        debug!("connection secret updated");
    }
}

struct RabbitChannelCallback;

#[async_trait]
impl ChannelCallback for RabbitChannelCallback {
    async fn close(
        &mut self,
        _channel: &Channel,
        _close: amqprs::CloseChannel,
    ) -> Result<(), amqprs::error::Error> {
        debug!("channel {:?} closed", _close);
        Ok(())
    }

    async fn cancel(
        &mut self,
        _channel: &Channel,
        _cancel: Cancel,
    ) -> Result<(), amqprs::error::Error> {
        debug!("channel {:?} cancel", _cancel);
        Ok(())
    }

    async fn flow(
        &mut self,
        _channel: &Channel,
        _flow: bool,
    ) -> Result<bool, amqprs::error::Error> {
        debug!("channel {:?} flow", _flow);
        Ok(true)
    }

    async fn publish_ack(&mut self, _channel: &Channel, _ack: Ack) {}

    async fn publish_nack(&mut self, _channel: &Channel, _nack: Nack) {}

    async fn publish_return(
        &mut self,
        _channel: &Channel,
        _return: Return,
        _props: BasicProperties,
        _content: Vec<u8>,
    ) {
    }
}
