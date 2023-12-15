use async_trait::async_trait;
use prost::Message;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uprotocol_sdk::{
    rpc::{RpcClient, RpcClientResult, RpcMapperError},
    transport::datamodel::UTransport,
    uprotocol::UCode,
    uprotocol::{UAttributes, UEntity, UMessage, UPayload, UStatus, UUri},
    uri::{
        serializer::{LongUriSerializer, UriSerializer},
        validator::UriValidator,
    },
};
use zenoh::config::Config;
use zenoh::prelude::r#async::*;
use zenoh::subscriber::Subscriber;

pub struct ZenohListener {}
pub struct ULink {
    session: Arc<Session>,
    map: Arc<Mutex<HashMap<String, Subscriber<'static, ()>>>>,
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
        Ok(ULink {
            session: Arc::new(session),
            map: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn to_zenoh_key(uri: &UUri) -> Result<String, UStatus> {
        // uProtocol Uri format: https://github.com/eclipse-uprotocol/uprotocol-spec/blob/6f0bb13356c0a377013bdd3342283152647efbf9/basics/uri.adoc#11-rfc3986
        // up://<user@><device>.<domain><:port>/<ue_name>/<ue_version>/<resource|rpc.method><#message>
        //            UAuthority               /        UEntity       /           UResource
        let Ok(uri_string) = LongUriSerializer::serialize(uri) else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to transform to Zenoh key",
            ));
        };
        // TODO: Able to be optimized without too many copy
        let mut zenoh_key = String::from("zenoh_uprotocol");
        if uri.authority.is_some() {
            zenoh_key += "/";
        }
        // TODO: Check whether these characters are all used in UUri.
        zenoh_key += &uri_string
            .replace('*', "\\8")
            .replace('$', "\\4")
            .replace('?', "\\0")
            .replace('#', "\\3")
            .replace("//", "\\/");
        Ok(zenoh_key)
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
        payload: UPayload,
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

        // Get Zenoh key
        let zenoh_key = ULink::to_zenoh_key(&topic)?;

        // TODO: Get payload
        let mut buf = vec![];
        payload.encode(&mut buf).unwrap();

        // Send data
        if self
            .session
            .put(&zenoh_key, buf)
            // TODO: Should be discussed (should be protobuf)
            .encoding(Encoding::APP_CUSTOM)
            .res()
            .await
            .is_err()
        {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to send with Zenoh",
            ));
        }

        Ok(())
    }

    async fn register_listener(
        &self,
        topic: UUri,
        listener: Box<dyn Fn(UMessage) + Send + Sync + 'static>,
    ) -> Result<String, UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // Get Zenoh key
        let zenoh_key = ULink::to_zenoh_key(&topic)?;

        if let Ok(subscriber) = self
            .session
            .declare_subscriber(&zenoh_key)
            .callback_mut(move |sample| {
                // TODO: Fill the Attributes
                let v = sample.payload.contiguous();
                let payload: UPayload = Message::decode(&*v).unwrap();
                let msg = UMessage {
                    source: Some(topic.clone()),
                    attributes: None,
                    payload: Some(payload),
                };
                listener(msg);
            })
            .res()
            .await
        {
            self.map
                .lock()
                .unwrap()
                .insert(zenoh_key.clone(), subscriber);
        } else {
            return Err(UStatus::fail_with_code(
                UCode::Internal,
                "Unable to register callback with Zenoh",
            ));
        }

        // TODO: Need to assign special string
        Ok(zenoh_key)
    }

    async fn unregister_listener(&self, topic: UUri, listener: &str) -> Result<(), UStatus> {
        // Do the validation
        if UriValidator::validate(&topic).is_err() {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Invalid topic",
            ));
        }

        // TODO: Check whether we still need topic or not
        self.map.lock().unwrap().remove(listener);
        Ok(())
    }
}
