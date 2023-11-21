use std::{future::Future, pin::Pin};

use cloudevents::Event;
use uprotocol_sdk::{
    cloudevent::{
        builder::UCloudEventBuilder,
        datamodel::{Priority, UCloudEventAttributesBuilder},
        serializer::{CloudEventJsonSerializer, CloudEventSerializer},
    },
    rpc::{RpcClient, RpcClientResult, RpcMapperError},
    transport::{
        datamodel::{
            UAttributes, UListener, UMessageType, UPayload, UPriority, UStatus, UTransport,
        },
        validator::Validators,
    },
    uri::{
        datamodel::{UEntity, UUri},
        serializer::{LongUriSerializer, UriSerializer},
        validator::UriValidator,
    },
};

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

    fn send(&self, topic: UUri, _payload: UPayload, attributes: UAttributes) -> UStatus {
        if let UStatus::Fail(fail_status) =
            Validators::get_validator(&attributes).validate(&attributes)
        {
            return UStatus::Fail(fail_status);
        }
        match attributes.message_type {
            UMessageType::Publish => {
                if let UStatus::Fail(fail_status) = UriValidator::validate(&topic) {
                    return UStatus::Fail(fail_status);
                }
                // TODO: Send Zenoh data
                UStatus::ok_with_id("successfully publish value to zenoh_up")
            }
            UMessageType::Request => {
                if let UStatus::Fail(fail_status) = UriValidator::validate_rpc_method(&topic) {
                    return UStatus::Fail(fail_status);
                }
                // TODO: Send Zenoh request
                UStatus::ok_with_id("successfully send rpc request to zenoh")
            }
            UMessageType::Response => {
                if let UStatus::Fail(fail_status) = UriValidator::validate_rpc_response(&topic) {
                    return UStatus::Fail(fail_status);
                }
                // TODO: Send Zenoh response
                UStatus::ok_with_id("successfully send rpc response to zenoh")
            }
        }
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
    fn to_priority(priority: UPriority) -> Priority {
        match priority {
            UPriority::Low => Priority::Low,
            UPriority::MultimediaStreaming => Priority::MultimediaStreaming,
            UPriority::NetworkControl => Priority::NetworkControl,
            UPriority::Operations => Priority::Operations,
            UPriority::RealtimeInteractive => Priority::RealtimeInteractive,
            UPriority::Signaling => Priority::Signaling,
            UPriority::Standard => Priority::Standard,
        }
    }
    fn create_serialized_ce(
        uri: &UUri,
        payload: &UPayload,
        attributes: UAttributes,
    ) -> (Event, Vec<u8>) {
        let ce_attributes = UCloudEventAttributesBuilder::new()
            .with_priority(ZenohUtils::to_priority(attributes.priority))
            .with_ttl(attributes.ttl.unwrap())
            .with_token(attributes.token.unwrap())
            .build();
        let ce = match attributes.message_type {
            UMessageType::Publish => UCloudEventBuilder::publish(
                &LongUriSerializer::serialize(uri),
                &payload.to_any().unwrap(),
                &ce_attributes,
            ),
            UMessageType::Request => {
                let applicationuri_for_rpc = LongUriSerializer::serialize(&UUri::rpc_response(
                    uri.authority.clone(),
                    uri.entity.clone(),
                ));
                UCloudEventBuilder::request(
                    &applicationuri_for_rpc,
                    &LongUriSerializer::serialize(uri),
                    &payload.to_any().unwrap(),
                    &ce_attributes,
                )
            }
            UMessageType::Response => {
                let applicationuri_for_rpc = LongUriSerializer::serialize(&UUri::rpc_response(
                    uri.authority.clone(),
                    uri.entity.clone(),
                ));
                let request_id = attributes.id.to_string();
                UCloudEventBuilder::response(
                    &applicationuri_for_rpc,
                    &LongUriSerializer::serialize(uri),
                    &request_id,
                    &payload.to_any().unwrap(),
                    &ce_attributes,
                )
            }
        };
        let json_serializer = CloudEventJsonSerializer;
        let serialized_bytes = json_serializer.serialize(&ce).unwrap();
        (ce, serialized_bytes)
    }
    //fn replace_special_chars() {}
    //fn add_endpoint() {}
    //fn send_data_to_zenoh(&self) {}
    //fn send_rpc_request_zenoh(&self) {}
    //fn register_rpc(&self) {}
}
