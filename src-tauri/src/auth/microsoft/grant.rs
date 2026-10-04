//! A validated OAuth grant, available before dependent Xbox/Minecraft requests.
use super::{error::Stage, MicrosoftError};

pub(crate) struct MicrosoftGrant {
    access_token: String,
    refresh_token: String,
    stage: Stage,
}

impl MicrosoftGrant {
    pub(crate) fn new(access_token: String, refresh_token: String) -> Result<Self, MicrosoftError> {
        Self::at_stage(access_token, refresh_token, Stage::Refresh)
    }

    pub(super) fn at_stage(
        access_token: String,
        refresh_token: String,
        stage: Stage,
    ) -> Result<Self, MicrosoftError> {
        for (field, value) in [
            ("access token", &access_token),
            ("refresh token", &refresh_token),
        ] {
            if value.trim().is_empty() {
                return Err(MicrosoftError::Protocol { stage, field });
            }
        }
        Ok(Self {
            access_token,
            refresh_token,
            stage,
        })
    }

    pub(crate) fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    pub(super) fn into_token(self) -> super::oauth::MsToken {
        super::oauth::MsToken {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
        }
    }

    pub(super) fn stage(&self) -> Stage {
        self.stage
    }
}

// Grants contain secrets, including in errors returned by test assertions.
impl std::fmt::Debug for MicrosoftGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftGrant").finish_non_exhaustive()
    }
}
