use crate::ipc::ApiError;
use crate::minecraft::bedrock::{self, BedrockClientStatus};

#[taurpc::procedures(path = "bedrock")]
pub trait BedrockApi {
    async fn get_bedrock_client_status() -> Result<BedrockClientStatus, ApiError>;
    async fn launch_bedrock_client() -> Result<(), ApiError>;
    async fn install_bedrock_client() -> Result<(), ApiError>;
    async fn open_bedrock_folder(kind: String) -> Result<(), ApiError>;
}

#[derive(Clone)]
pub struct BedrockApiImpl;

#[taurpc::resolvers]
impl BedrockApi for BedrockApiImpl {
    async fn get_bedrock_client_status(self) -> Result<BedrockClientStatus, ApiError> {
        Ok(bedrock::detect_bedrock_status())
    }

    async fn launch_bedrock_client(self) -> Result<(), ApiError> {
        bedrock::launch_bedrock().map_err(ApiError::from)
    }

    async fn install_bedrock_client(self) -> Result<(), ApiError> {
        bedrock::install_bedrock().map_err(ApiError::from)
    }

    async fn open_bedrock_folder(self, kind: String) -> Result<(), ApiError> {
        bedrock::open_bedrock_folder(&kind).map_err(ApiError::from)
    }
}
