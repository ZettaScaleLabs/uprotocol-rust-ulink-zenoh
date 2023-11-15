use std::future::Future;
use std::pin::Pin;

use uprotocol_sdk::rpc::{RpcClient, RpcClientResult, RpcMapperError};
use uprotocol_sdk::transport::datamodel::{UAttributes, UListener, UPayload, UStatus, UTransport};
use uprotocol_sdk::uri::datamodel::{UEntity, UUri};

pub struct ZenohListener {}

impl UListener for ZenohListener {
    fn on_receive(&self, _topic: UUri, _payload: UPayload, _attributes: UAttributes) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }
}

pub struct Zenoh {}

impl RpcClient for Zenoh {
    fn invoke_method(
        _topic: UUri,
        _payload: UPayload,
        _attributes: UAttributes,
    ) -> Pin<Box<dyn Future<Output = RpcClientResult>>> {
        Box::pin(async { Err(RpcMapperError::UnknownType("Not implemented".to_string())) })
    }
}

impl UTransport for Zenoh {
    type L = ZenohListener;

    fn register(&self, _uentity: UEntity, _token: &[u8]) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn send(&self, _topic: UUri, _payload: UPayload, _attributes: UAttributes) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn register_listener(&self, _topic: UUri, _listener: ZenohListener) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }

    fn unregister_listener(&self, _topic: UUri, _listener: ZenohListener) -> UStatus {
        UStatus::fail_with_msg("Not implemented")
    }
}

#[allow(dead_code)]
struct ZenohUtils {}

#[allow(dead_code)]
impl ZenohUtils {
    fn replace_special_chars() {}
    fn create_serialized_ce() {}
    fn add_endpoint() {}
    fn send_data_to_zenoh(&self) {}
    fn send_rpc_request_zenoh(&self) {}
    fn register_rpc(&self) {}
}
