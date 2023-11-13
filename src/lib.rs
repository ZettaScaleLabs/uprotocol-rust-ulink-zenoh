use std::future::Future;
use std::pin::Pin;

use uprotocol_sdk::rpc::{RpcClient, RpcClientResult, RpcMapperError};
use uprotocol_sdk::transport::datamodel::{UAttributes, UListener, UPayload, UStatus, UTransport};
use uprotocol_sdk::uri::datamodel::{UEntity, UUri};

pub struct Zenoh {}

impl RpcClient for Zenoh {
    fn invoke_method(
        topic: UUri,
        payload: UPayload,
        attributes: UAttributes,
    ) -> Pin<Box<dyn Future<Output = RpcClientResult>>> {
        Box::pin(async { Err(RpcMapperError::UnknownType("Not implemented".to_string())) })
    }
}

impl UTransport for Zenoh {
    fn register(&self, uentity: UEntity, token: &[u8]) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn send(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn register_listener(&self, topic: UUri, listener: Box<dyn UListener>) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn unregister_listener(&self, topic: UUri, listener: Box<dyn UListener>) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }
}

struct ZenohListener {}
impl UListener for ZenohListener {
    fn on_receive(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }
}

struct ZenohUtils {}

impl ZenohUtils {
    fn replace_special_chars() {}
    fn create_serialized_ce() {}
    fn add_endpoint() {}
    fn send_data_to_zenoh(&self) {}
    fn send_rpc_request_zenoh(&self) {}
    fn register_rpc(&self) {}
}
