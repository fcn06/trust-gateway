use trust_model::TrustEnvelope;

pub struct NatsTransport {
    client: async_nats::Client,
}

impl NatsTransport {
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }

    pub async fn publish_envelope<T: serde::Serialize>(
        &self,
        subject: &str,
        envelope: &TrustEnvelope<T>,
    ) -> Result<(), anyhow::Error> {
        let payload = serde_json::to_vec(envelope)?;
        self.client
            .publish(subject.to_string(), payload.into())
            .await?;
        Ok(())
    }
}
