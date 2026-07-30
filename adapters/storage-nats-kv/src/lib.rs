pub struct NatsKvStorage {
    store: async_nats::jetstream::kv::Store,
}

impl NatsKvStorage {
    pub fn new(store: async_nats::jetstream::kv::Store) -> Self {
        Self { store }
    }

    /// Check and consume a grant nonce atomically using _ separator (RULE 020).
    pub async fn consume_nonce(&self, nonce: &str) -> Result<bool, anyhow::Error> {
        let key = format!("nonce_{nonce}");
        match self.store.get(&key).await {
            Ok(Some(_)) => Ok(false), // Already consumed!
            _ => {
                self.store.put(&key, "consumed".into()).await?;
                Ok(true)
            }
        }
    }
}
