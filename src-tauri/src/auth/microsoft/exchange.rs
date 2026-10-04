use super::{
    client::MicrosoftSession,
    error::{MicrosoftError, Stage},
    oauth::MsToken,
    MicrosoftGrant, MinecraftSession,
};
use crate::account::provider::Cancelled;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxToken {
    token: String,
    display_claims: Claims,
}
#[derive(Deserialize)]
struct Claims {
    xui: Vec<UserInfo>,
}
#[derive(Deserialize)]
struct UserInfo {
    uhs: String,
}
#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    code: u64,
}
#[derive(Deserialize)]
struct MinecraftToken {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
pub(super) struct McProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<McSkin>,
}
#[derive(Deserialize)]
pub(super) struct McSkin {
    pub state: String,
    pub url: String,
}

struct XboxIdentity {
    token: String,
    user_hash: String,
}

impl MicrosoftSession {
    pub(crate) async fn exchange_grant(
        &self,
        grant: MicrosoftGrant,
        cancelled: Cancelled<'_>,
    ) -> Result<MinecraftSession, MicrosoftError> {
        let stage = grant.stage();
        self.exchange(grant.into_token(), stage, cancelled).await
    }

    pub(super) async fn exchange(
        &self,
        ms: MsToken,
        token_stage: Stage,
        cancelled: Cancelled<'_>,
    ) -> Result<MinecraftSession, MicrosoftError> {
        Self::nonempty(&ms.access_token, token_stage, "access token")?;
        Self::nonempty(&ms.refresh_token, token_stage, "refresh token")?;
        check_cancelled(cancelled)?;
        let xbox = self.xbox_token(&ms.access_token).await?;
        check_cancelled(cancelled)?;
        let identity = self.xbox_identity(&xbox.token).await?;
        check_cancelled(cancelled)?;
        let minecraft = self.minecraft_token(&identity).await?;
        // Count lifetime from token receipt, rather than extending it by profile I/O.
        let expires_at = self
            .clock
            .unix_seconds()
            .saturating_add(minecraft.expires_in);
        check_cancelled(cancelled)?;
        let profile = self.minecraft_profile(&minecraft.access_token).await?;
        check_cancelled(cancelled)?;
        Ok(MinecraftSession {
            access_token: minecraft.access_token,
            expires_at,
            refresh_token: ms.refresh_token,
            uuid: profile.id,
            username: profile.name,
            skin_url: profile
                .skins
                .into_iter()
                .find(|skin| skin.state == "ACTIVE")
                .map(|skin| skin.url),
        })
    }

    async fn xbox_token(&self, access: &str) -> Result<XboxToken, MicrosoftError> {
        let request = self.client.post(&self.endpoints.xbox).json(&json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={access}") },
            "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT",
        }));
        let xbox: XboxToken =
            Self::decode(self.send(request, Stage::XboxLive).await?, Stage::XboxLive).await?;
        Self::nonempty(&xbox.token, Stage::XboxLive, "token")?;
        Ok(xbox)
    }

    async fn xbox_identity(&self, token: &str) -> Result<XboxIdentity, MicrosoftError> {
        let request = self.client.post(&self.endpoints.xsts).json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [token] },
            "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT",
        }));
        let response = self.send(request, Stage::Xsts).await?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            let error: XstsError = Self::body(response, Stage::Xsts).await?;
            return Err(MicrosoftError::xsts(error.code));
        }
        let xsts: XboxToken = Self::decode(response, Stage::Xsts).await?;
        let user = xsts
            .display_claims
            .xui
            .first()
            .ok_or(MicrosoftError::Protocol {
                stage: Stage::Xsts,
                field: "Xbox user",
            })?;
        Self::nonempty(&user.uhs, Stage::Xsts, "Xbox user hash")?;
        Self::nonempty(&xsts.token, Stage::Xsts, "token")?;
        Ok(XboxIdentity {
            token: xsts.token,
            user_hash: user.uhs.clone(),
        })
    }

    async fn minecraft_token(
        &self,
        identity: &XboxIdentity,
    ) -> Result<MinecraftToken, MicrosoftError> {
        let request = self
            .client
            .post(&self.endpoints.minecraft_login)
            .json(&json!({
                "identityToken": format!("XBL3.0 x={};{}", identity.user_hash, identity.token)
            }));
        let response = self.send(request, Stage::MinecraftLogin).await?;
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(MicrosoftError::AppRegistration);
        }
        let minecraft: MinecraftToken = Self::decode(response, Stage::MinecraftLogin).await?;
        Self::nonempty(
            &minecraft.access_token,
            Stage::MinecraftLogin,
            "access token",
        )?;
        Ok(minecraft)
    }

    async fn minecraft_profile(&self, access: &str) -> Result<McProfile, MicrosoftError> {
        let request = self.client.get(&self.endpoints.profile).bearer_auth(access);
        let response = self.send(request, Stage::Profile).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(MicrosoftError::MissingMinecraftProfile);
        }
        let profile: McProfile = Self::decode(response, Stage::Profile).await?;
        Self::nonempty(&profile.id, Stage::Profile, "profile ID")?;
        Self::nonempty(&profile.name, Stage::Profile, "profile name")?;
        Ok(profile)
    }
}

fn check_cancelled(cancelled: Cancelled<'_>) -> Result<(), MicrosoftError> {
    if cancelled() {
        return Err(MicrosoftError::Cancelled);
    }
    Ok(())
}
