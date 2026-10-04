use reqwest::StatusCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    DeviceCode,
    DeviceToken,
    Refresh,
    XboxLive,
    Xsts,
    MinecraftLogin,
    Profile,
}
impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DeviceCode => "Microsoft device code",
            Self::DeviceToken => "Microsoft sign-in",
            Self::Refresh => "Microsoft refresh",
            Self::XboxLive => "Xbox Live",
            Self::Xsts => "Xbox authorization",
            Self::MinecraftLogin => "Minecraft sign-in",
            Self::Profile => "Minecraft profile",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MicrosoftError {
    #[error("Microsoft sign-in isn't configured in this build")]
    NotConfigured,
    #[error("Microsoft sign-in cancelled")]
    Cancelled,
    #[error("The code expired. Please try again.")]
    CodeExpired,
    #[error("Sign-in was declined")]
    Declined,
    #[error("Microsoft did not recognize the sign-in code; please try again")]
    InvalidCode,
    #[error("Your Microsoft session expired, please sign in again")]
    RefreshExpired,
    #[error("Could not reach {stage}")]
    Transport {
        stage: Stage,
        #[source]
        source: reqwest::Error,
    },
    #[error("Unexpected response from {stage}")]
    Response {
        stage: Stage,
        #[source]
        source: reqwest::Error,
    },
    #[error("Invalid {field} in the {stage} response")]
    Protocol { stage: Stage, field: &'static str },
    #[error("{stage} refused the request (HTTP {status})")]
    Rejected { stage: Stage, status: StatusCode },
    #[error("This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one.")]
    MissingXboxProfile,
    #[error("Xbox Live isn't available in your country")]
    XboxRegion,
    #[error("This account needs adult verification on xbox.com")]
    AdultVerification,
    #[error("This is a child account. An adult must add it to a Microsoft family first.")]
    ChildAccount,
    #[error("Xbox Live refused the sign-in (error {code})")]
    XstsRejected { code: u64 },
    #[error("Minecraft services rejected this launcher's app registration")]
    AppRegistration,
    #[error("This Microsoft account doesn't own Minecraft: Java Edition, or hasn't set up a profile on minecraft.net yet")]
    MissingMinecraftProfile,
}

impl MicrosoftError {
    pub(super) fn xsts(code: u64) -> Self {
        match code {
            2148916233 => Self::MissingXboxProfile,
            2148916235 => Self::XboxRegion,
            2148916236 | 2148916237 => Self::AdultVerification,
            2148916238 => Self::ChildAccount,
            code => Self::XstsRejected { code },
        }
    }
}
