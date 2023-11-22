use std::{future::Future, pin::Pin};

use cloudevents::Event;
use uprotocol_sdk::{
    cloudevent::{
        builder::UCloudEventBuilder,
        datamodel::{Priority, UCloudEvent, UCloudEventAttributesBuilder},
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
use zenoh::config::Config;
use zenoh::prelude::sync::*;

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

    fn send(&self, topic: UUri, payload: UPayload, attributes: UAttributes) -> UStatus {
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
                let (ce, serialized_str) =
                    ZenohUtils::create_serialized_ce(&topic, &payload, attributes);
                ZenohUtils::send_data_to_zenoh(
                    &UCloudEvent::get_source(&ce).unwrap(),
                    serialized_str,
                );
                UStatus::ok_with_id("successfully publish value to Zenoh")
            }
            UMessageType::Request => {
                if let UStatus::Fail(fail_status) = UriValidator::validate_rpc_method(&topic) {
                    return UStatus::Fail(fail_status);
                }
                // TODO: Send Zenoh request
                UStatus::ok_with_id("successfully send rpc request to Zenoh")
            }
            UMessageType::Response => {
                if let UStatus::Fail(fail_status) = UriValidator::validate_rpc_response(&topic) {
                    return UStatus::Fail(fail_status);
                }
                // TODO: Send Zenoh response
                UStatus::ok_with_id("successfully send rpc response to Zenoh")
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
        // TODO: Should we encode with base64
        (ce, serialized_bytes)
    }

    fn replace_special_chars(topic: &str) -> String {
        // https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md#source-1
        // TODO: Should we also replace '#' and '?'
        // Replace // with ==
        let topic = topic.replace("//", "==");
        // Remove leading and trailing /
        topic.trim_matches('/').to_string()
    }

    fn send_data_to_zenoh(topic: &str, data_to_send: Vec<u8>) {
        let new_topic = ZenohUtils::replace_special_chars(topic);
        let session = zenoh::open(Config::default()).res().unwrap();
        session.put(&new_topic, data_to_send).res().unwrap();
        session.close().res().unwrap();
    }

    // TODO
    //fn add_endpoint() {}
    //fn send_rpc_request_zenoh(&self) {}
    //fn register_rpc(&self) {}
}
