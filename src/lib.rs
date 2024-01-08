//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use async_trait::async_trait;
use prost::Message;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uprotocol_sdk::{
    rpc::{RpcClient, RpcClientResult, RpcMapperError, RpcServer},
    transport::{datamodel::UTransport, validator::Validators},
    uprotocol::{
        Data, UAttributes, UCode, UEntity, UMessage, UMessageType, UPayload, UPayloadFormat,
        UStatus, UUri, Uuid,
    },
    uri::{
        serializer::{LongUriSerializer, UriSerializer},
        validator::UriValidator,
    },
};
use zenoh::{
    config::Config,
    prelude::{r#async::*, Sample},
    queryable::{Query, Queryable},
    sample::AttachmentBuilder,
    subscriber::Subscriber,
};

pub struct ZenohListener {}
pub struct ULinkZenoh {
    session: Arc<Session>,
    subscriber_map: Arc<Mutex<HashMap<String, Subscriber<'static, ()>>>>,
    queryable_map: Arc<Mutex<HashMap<String, Queryable<'static, ()>>>>,
    query_map: Arc<Mutex<HashMap<String, Query>>>,
}

impl ULinkZenoh {
    /// # Errors
    /// Will return `Err` if unable to create Zenoh session
    pub async fn new(config: Config) -> Result<ULinkZenoh, UStatus> {
        let Ok(session) = zenoh::open(config).res().await else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to open Zenoh session",
            ));
        };
        Ok(ULinkZenoh {
            session: Arc::new(session),
            subscriber_map: Arc::new(Mutex::new(HashMap::new())),
            queryable_map: Arc::new(Mutex::new(HashMap::new())),
            query_map: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn to_zenoh_key_string(uri: &UUri) -> Result<String, UStatus> {
        // uProtocol Uri format: https://github.com/eclipse-uprotocol/uprotocol-spec/blob/6f0bb13356c0a377013bdd3342283152647efbf9/basics/uri.adoc#11-rfc3986
        // up://<user@><device>.<domain><:port>/<ue_name>/<ue_version>/<resource|rpc.method><#message>
        //            UAuthority               /        UEntity       /           UResource
        let Ok(mut uri_string) = LongUriSerializer::serialize(uri) else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to transform to Zenoh key",
            ));
        };
        if uri_string.starts_with('/') {
            let _ = uri_string.remove(0);
        }

        // TODO: Check whether these characters are all used in UUri.
        // TODO: We should have the # and ? in the attachment instead of Zenoh key
        let zenoh_key = uri_string
            .replace('*', "\\8")
            .replace('$', "\\4")
            .replace('?', "\\0")
            .replace('#', "\\3")
            .replace("//", "\\/");
        Ok(zenoh_key)
    }

    // TODO: We need a standard way in uprotocol-rust to change UUID to String
    fn uuid_to_string(uuid: &Uuid) -> String {
        format!("{}:{}", uuid.msb, uuid.lsb)
    }
}

#[async_trait]
impl RpcClient for ULinkZenoh {
    async fn invoke_method(
        &self,
        topic: UUri,
        payload: UPayload,
        attributes: UAttributes,
    ) -> RpcClientResult {
        // Validate UUri
        if UriValidator::validate(&topic).is_err() {
            return Err(RpcMapperError::UnexpectedError(String::from("Wrong UUri")));
        }
        // Validate UAttributes
        // TODO: Check validate sink (validate_sink)
        //{
        //    // TODO: Check why the validator doesn't have Send
        //    let validator = Validators::Request.validator();
        //    if let Err(e) = validator.validate(&attributes) {
        //        return Err(RpcMapperError::UnexpectedError(format!(
        //            "Wrong UAttributes {e:?}",
        //        )));
        //    }
        //}

        // Get Zenoh key
        let Ok(zenoh_key) = ULinkZenoh::to_zenoh_key_string(&topic) else {
            return Err(RpcMapperError::UnexpectedError(String::from(
                "Unable to transform to Zenoh key",
            )));
        };

        // Get the data from UPayload
        let Some(Data::Value(buf)) = payload.data else {
            // TODO: Assume we only have Value here, no reference for shared memory
            return Err(RpcMapperError::InvalidPayload(String::from(
                "Wrong UPayload",
            )));
        };

        // Serialized UAttributes into protobuf
        // TODO: Should we map priority into Zenoh priority?
        let mut attr = vec![];
        let Ok(()) = attributes.encode(&mut attr) else {
            return Err(RpcMapperError::ProtobufError(String::from(
                "Unable to encode UAttributes",
            )));
        };

        // Add attachment and payload
        let mut attachment = AttachmentBuilder::new();
        attachment.insert("uattributes", attr.as_slice());
        let value = Value::new(buf.into()).encoding(Encoding::WithSuffix(
            KnownEncoding::AppCustom,
            payload.format.to_string().into(),
        ));
        // TODO: Query should support .encoding
        // TODO: Adjust the timeout
        let getbuilder = self
            .session
            .get(&zenoh_key)
            .with_value(value)
            .with_attachment(attachment.build())
            .target(QueryTarget::BestMatching)
            .timeout(Duration::from_millis(1000));

        // Send the query
        let Ok(replies) = getbuilder.res().await else {
            return Err(RpcMapperError::UnexpectedError(String::from(
                "Error while sending Zenoh query",
            )));
        };

        let Ok(reply) = replies.recv_async().await else {
            return Err(RpcMapperError::UnexpectedError(String::from(
                "Error while receiving Zenoh reply",
            )));
        };
        match reply.sample {
            Ok(sample) => {
                let Ok(encoding) = sample.value.encoding.suffix().parse::<i32>() else {
                    return Err(RpcMapperError::UnexpectedError(String::from(
                        "Error while parsing Zenoh encoding",
                    )));
                };
                Ok(UPayload {
                    length: Some(0),
                    format: encoding,
                    data: Some(Data::Value(sample.payload.contiguous().to_vec())),
                })
            }
            Err(_) => Err(RpcMapperError::UnexpectedError(String::from(
                "Error while parsing Zenoh reply",
            ))),
        }
    }
}

#[async_trait]
impl RpcServer for ULinkZenoh {
    async fn register_rpc_listener(
        &self,
        method: UUri,
        listener: Box<dyn Fn(Result<UMessage, UStatus>) + Send + Sync + 'static>,
    ) -> Result<String, UStatus> {
        // Do the validation
        if UriValidator::validate(&method).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // Get Zenoh key
        let zenoh_key = ULinkZenoh::to_zenoh_key_string(&method)?;
        // Generate listener string for users to delete
        let mut hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        while self
            .queryable_map
            .lock()
            .unwrap()
            .contains_key(&hashmap_key)
        {
            hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        }

        let query_map = self.query_map.clone();
        // Setup callback
        let callback = move |query: Query| {
            // Create UAttribute
            let Some(attachment) = query.attachment() else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to get attachment",
                )));
                return;
            };
            let Some(attribute) = attachment.get(&"uattributes".as_bytes()) else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to get uattributes",
                )));
                return;
            };
            let u_attribute: UAttributes = if let Ok(attr) = Message::decode(&*attribute) {
                attr
            } else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to decode attribute",
                )));
                return;
            };
            // Create UPayload
            let u_payload = match query.value() {
                Some(value) => {
                    let Ok(encoding) = value.encoding.suffix().parse::<i32>() else {
                        listener(Err(UStatus::fail_with_code(
                            UCode::Internal,
                            "Unable to get payload encoding",
                        )));
                        return;
                    };
                    UPayload {
                        length: Some(0),
                        format: encoding,
                        data: Some(Data::Value(value.payload.contiguous().to_vec())),
                    }
                }
                None => UPayload {
                    length: Some(0),
                    format: UPayloadFormat::UpayloadFormatUnspecified as i32,
                    data: None,
                },
            };
            // Create UMessage
            let msg = UMessage {
                source: Some(method.clone()),
                attributes: Some(u_attribute.clone()),
                payload: Some(u_payload),
            };
            if let Some(reqid) = u_attribute.reqid {
                query_map
                    .lock()
                    .unwrap()
                    .insert(ULinkZenoh::uuid_to_string(&reqid), query);
            } else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "The request is without reqid in UAttributes",
                )));
                return;
            }
            listener(Ok(msg));
        };
        if let Ok(queryable) = self
            .session
            .declare_queryable(&zenoh_key)
            .callback_mut(callback)
            .res()
            .await
        {
            self.queryable_map
                .lock()
                .unwrap()
                .insert(hashmap_key.clone(), queryable);
        } else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to register callback with Zenoh",
            ));
        }

        Ok(hashmap_key)
    }
    async fn unregister_rpc_listener(&self, method: UUri, listener: &str) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&method).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }
        // TODO: Check whether we still need method or not (Compare method with listener?)

        if !self.queryable_map.lock().unwrap().contains_key(listener) {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Listener doesn't exist",
            ));
        }

        self.queryable_map.lock().unwrap().remove(listener);
        Ok(())
    }
}

#[async_trait]
impl UTransport for ULinkZenoh {
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
        payload: UPayload,
        attributes: UAttributes,
    ) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }
        // TODO: Validate UAttributes (We don't know whether attributes are Publish/Request/Response, so we can't check)

        // Get Zenoh key
        let zenoh_key = ULinkZenoh::to_zenoh_key_string(&topic)?;

        // Check the type of UAttributes (Publish / Request / Response)
        // TODO: Create function for different types
        match UMessageType::try_from(attributes.r#type) {
            Ok(UMessageType::UmessageTypePublish) => {
                // Get the data from UPayload
                let Some(Data::Value(buf)) = payload.data else {
                    // TODO: Assume we only have Value here, no reference for shared memory
                    return Err(UStatus::fail_with_code(
                        UCode::InvalidArgument,
                        "Invalid data",
                    ));
                };

                // Serialized UAttributes into protobuf
                // TODO: Should we map priority into Zenoh priority?
                let mut attr = vec![];
                let Ok(()) = attributes.encode(&mut attr) else {
                    return Err(UStatus::fail_with_code(
                        UCode::InvalidArgument,
                        "Unable to encode UAttributes",
                    ));
                };

                // Add attachment and payload
                let mut attachment = AttachmentBuilder::new();
                attachment.insert("uattributes", attr.as_slice());
                let putbuilder = self
                    .session
                    .put(&zenoh_key, buf)
                    .encoding(Encoding::WithSuffix(
                        KnownEncoding::AppCustom,
                        payload.format.to_string().into(),
                    ))
                    .with_attachment(attachment.build());

                // Send data
                if putbuilder.res().await.is_err() {
                    return Err(UStatus::fail_with_code(
                        UCode::Internal,
                        "Unable to send with Zenoh",
                    ));
                }

                Ok(())
            }
            Ok(UMessageType::UmessageTypeResponse) => {
                // TODO: Get the query from map with UUID string in UAttributes
                // TODO: Response
                // Get the data from UPayload
                let Some(Data::Value(buf)) = payload.data else {
                    // TODO: Assume we only have Value here, no reference for shared memory
                    return Err(UStatus::fail_with_code(
                        UCode::InvalidArgument,
                        "Invalid data",
                    ));
                };

                // Serialized UAttributes into protobuf
                // TODO: Should we map priority into Zenoh priority?
                let mut attr = vec![];
                let Ok(()) = attributes.encode(&mut attr) else {
                    return Err(UStatus::fail_with_code(
                        UCode::InvalidArgument,
                        "Unable to encode UAttributes",
                    ));
                };
                // TODO: Do not use unwrap()
                let reqid = ULinkZenoh::uuid_to_string(&attributes.reqid.unwrap());

                // Add attachment and payload
                let mut attachment = AttachmentBuilder::new();
                attachment.insert("uattributes", attr.as_slice());
                // Send back query
                let value = Value::new(buf.into()).encoding(Encoding::WithSuffix(
                    KnownEncoding::AppCustom,
                    payload.format.to_string().into(),
                ));
                let reply = Ok(Sample::new(
                    KeyExpr::new(ULinkZenoh::to_zenoh_key_string(&topic).unwrap()).unwrap(),
                    value,
                ));
                // TODO: Add attachment
                // TODO: Do not use unwrap
                let query = self.query_map.lock().unwrap().get(&reqid).unwrap().clone();

                // Send data
                if query.reply(reply).res().await.is_err() {
                    return Err(UStatus::fail_with_code(
                        UCode::Internal,
                        "Unable to reply with Zenoh",
                    ));
                }

                Ok(())
            }
            _ => Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Wrong Message type in UAttributes",
            )),
        }
    }

    async fn register_listener(
        &self,
        topic: UUri,
        listener: Box<dyn Fn(Result<UMessage, UStatus>) + Send + Sync + 'static>,
    ) -> Result<String, UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // Get Zenoh key
        let zenoh_key = ULinkZenoh::to_zenoh_key_string(&topic)?;
        // Generate listener string for users to delete
        let mut hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        while self
            .subscriber_map
            .lock()
            .unwrap()
            .contains_key(&hashmap_key)
        {
            hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        }

        // Setup callback
        let callback = move |sample: Sample| {
            // Create UAttribute
            let Some(attachment) = sample.attachment() else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to get attachment",
                )));
                return;
            };
            let Some(attribute) = attachment.get(&"uattributes".as_bytes()) else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to get uattributes",
                )));
                return;
            };
            let Ok(u_attribute) = Message::decode(&*attribute) else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to decode attribute",
                )));
                return;
            };
            // Create UPayload
            let Ok(encoding) = sample.encoding.suffix().parse::<i32>() else {
                listener(Err(UStatus::fail_with_code(
                    UCode::Internal,
                    "Unable to get payload encoding",
                )));
                return;
            };
            let u_payload = UPayload {
                length: Some(0),
                format: encoding,
                data: Some(Data::Value(sample.payload.contiguous().to_vec())),
            };
            // Create UMessage
            let msg = UMessage {
                source: Some(topic.clone()),
                attributes: Some(u_attribute),
                payload: Some(u_payload),
            };
            listener(Ok(msg));
        };
        if let Ok(subscriber) = self
            .session
            .declare_subscriber(&zenoh_key)
            .callback_mut(callback)
            .res()
            .await
        {
            self.subscriber_map
                .lock()
                .unwrap()
                .insert(hashmap_key.clone(), subscriber);
        } else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to register callback with Zenoh",
            ));
        }

        Ok(hashmap_key)
    }

    async fn unregister_listener(&self, topic: UUri, listener: &str) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }
        // TODO: Check whether we still need topic or not (Compare topic with listener?)

        if !self.subscriber_map.lock().unwrap().contains_key(listener) {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Listener doesn't exist",
            ));
        }

        self.subscriber_map.lock().unwrap().remove(listener);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uprotocol_sdk::uprotocol::{UEntity, UResource, UUri};

    #[test]
    fn test_to_zenoh_key_string() {
        // create uuri for test
        let uuri = UUri {
            entity: Some(UEntity {
                name: "body.access".to_string(),
                version_major: Some(1),
                ..Default::default()
            }),
            resource: Some(UResource {
                name: "door".to_string(),
                instance: Some("front_left".to_string()),
                message: Some("Door".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            ULinkZenoh::to_zenoh_key_string(&uuri).unwrap(),
            String::from("body.access/1/door.front_left\\3Door")
        );
    }
}
