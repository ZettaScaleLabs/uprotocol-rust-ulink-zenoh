use async_trait::async_trait;
use prost::Message;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uprotocol_sdk::{
    rpc::{RpcClient, RpcClientResult, RpcMapperError},
    transport::datamodel::UTransport,
    uprotocol::{UAttributes, UCode, UEntity, UMessage, UPayload, UStatus, UUri},
    uri::{
        serializer::{LongUriSerializer, UriSerializer},
        validator::UriValidator,
    },
};
use zenoh::config::Config;
use zenoh::prelude::r#async::*;
use zenoh::sample::AttachmentBuilder;
use zenoh::subscriber::Subscriber;

pub struct ZenohListener {}
pub struct ULinkZenoh {
    session: Arc<Session>,
    map: Arc<Mutex<HashMap<String, Subscriber<'static, ()>>>>,
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
        let mut zenoh_key = String::from("zenoh_uprotocol");
        if uri.authority.is_some() {
            zenoh_key.push('/');
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
impl RpcClient for ULinkZenoh {
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
        // TODO: Validate UAttributes (maybe without self)

        // Get Zenoh key
        let zenoh_key = ULinkZenoh::to_zenoh_key(&topic)?;

        // Serialize UPayload into protobuf
        let mut buf = vec![];
        let Ok(()) = payload.encode(&mut buf) else {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Unable to encode UPayload",
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
        let zenoh_key = ULinkZenoh::to_zenoh_key(&topic)?;
        // Generate listener string for users to delete
        let mut hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        while self.map.lock().unwrap().contains_key(&hashmap_key) {
            hashmap_key = format!("{}_{:X}", zenoh_key, rand::random::<u64>());
        }

        // Setup callback
        let callback = move |sample: Sample| {
            let Some(attachment) = sample.attachment() else {
                return; // Should not happen, so do nothing
            };
            let Some(attribute) = attachment.get(&"uattributes".as_bytes()) else {
                return; // Should not happen, so do nothing
            };
            let Ok(u_attribute) = Message::decode(&*attribute) else {
                return; // Should not happen, so do nothing
            };
            let payload = sample.payload.contiguous();
            let Ok(u_payload) = Message::decode(&*payload) else {
                return; // Should not happen, so do nothing
            };
            let msg = UMessage {
                source: Some(topic.clone()),
                attributes: Some(u_attribute),
                payload: Some(u_payload),
            };
            listener(msg);
        };
        if let Ok(subscriber) = self
            .session
            .declare_subscriber(&zenoh_key)
            .callback_mut(callback)
            .res()
            .await
        {
            self.map
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

        if !self.map.lock().unwrap().contains_key(listener) {
            return Err(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "Listener not exists",
            ));
        }

        self.map.lock().unwrap().remove(listener);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uprotocol_sdk::uprotocol::{UEntity, UResource, UUri};

    #[test]
    fn test_to_zenoh_key() {
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
            ULinkZenoh::to_zenoh_key(&uuri).unwrap(),
            String::from("zenoh_uprotocol/body.access/1/door.front_left\\3Door")
        );
    }
}
