mod exchange;
mod oauth;
mod support;

// Protocol tests compose both steps. Production callers checkpoint the grant
// between these operations through account::session.
impl super::MicrosoftSession {
    async fn refresh(
        &self,
        refresh: &str,
    ) -> Result<super::MinecraftSession, super::MicrosoftError> {
        let grant = self.refresh_grant(refresh).await?;
        self.exchange_grant(grant, &|| false).await
    }

    async fn finish(
        &self,
        code: &super::MicrosoftDeviceCode,
        cancelled: crate::account::provider::Cancelled<'_>,
    ) -> Result<super::MinecraftSession, super::MicrosoftError> {
        let grant = self.redeem(code, cancelled).await?;
        self.exchange_grant(grant, cancelled).await
    }
}
