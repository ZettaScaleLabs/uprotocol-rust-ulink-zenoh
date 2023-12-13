use async_trait::async_trait;
use uprotocol_sdk::{
    rpc::{RpcClient, RpcClientResult, RpcMapperError},
    transport::datamodel::UTransport,
    uprotocol::UCode,
    uprotocol::{UAttributes, UEntity, UMessage, UPayload, UStatus, UUri},
    uri::validator::UriValidator,
};
use zenoh::config::Config;
use zenoh::prelude::r#async::*;

pub struct ZenohListener {}
pub struct ULink {
    _session: Session,
}

impl ULink {
    /// # Errors
    /// Will return `Err` if unable to create Zenoh session
    pub async fn new() -> Result<ULink, UStatus> {
        let Ok(session) = zenoh::open(Config::default()).res().await else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to open Zenoh session",
            ));
        };
        Ok(ULink { _session: session })
    }
}

#[async_trait]
impl RpcClient for ULink {
    async fn invoke_method(
        topic: UUri,
        payload: UPayload,
        _attributes: UAttributes,
    ) -> RpcClientResult {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(RpcMapperError::UnexpectedError(String::from("Wrong UUri")));
        }
        // TODO: Validate UAttributes (maybe without self)

        // TODO: Not implemented
        Ok(payload)
    }
}

#[async_trait]
impl UTransport for ULink {
    async fn authenticate(&self, _entity: UEntity) -> Result<(), UStatus> {
        // TODO: Not implemented
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "Not implemented",
        ))
    }

    async fn send(
        &self,
        topic: UUri,
        _payload: UPayload,
        _attributes: UAttributes,
    ) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }
        // TODO: Validate UAttributes (maybe without self)

        // TODO: Not implemented
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "Not implemented",
        ))
    }

    async fn register_listener(
        &self,
        topic: UUri,
        _listener: Box<dyn Fn(UMessage) + Send + 'static>,
    ) -> Result<String, UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // TODO: Not implemented
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "Not implemented",
        ))
    }

    async fn unregister_listener(&self, topic: UUri, _listener: &str) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // TODO: Not implemented
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "Not implemented",
        ))
    }
}
