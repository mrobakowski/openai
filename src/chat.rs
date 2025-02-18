//! Given a chat conversation, the model will return a chat completion response.

use super::{models::ModelID, openai_post, ApiResponseOrError, Usage};
use derive_builder::Builder;
use futures::{future, Stream, StreamExt};
use openai_bootstrap::{authorization, BASE_URL};
use reqwest::{Client, Method};
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct ChatCompletion {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: ModelID,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ChatCompletionEvent {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: ModelID,
    pub choices: Vec<ChatCompletionChoiceDelta>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChatCompletionChoice {
    pub index: u64,
    pub message: ChatCompletionMessage,
    pub finish_reason: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ChatCompletionChoiceDelta {
    pub index: u64,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Delta {
    Role { role: ChatCompletionMessageRole },
    Content { content: String },
    EndOfStream {},
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChatCompletionMessage {
    /// The role of the author of this message.
    pub role: ChatCompletionMessageRole,
    /// The contents of the message
    pub content: String,
    /// The name of the user in a multi-user chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatCompletionMessageRole {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Builder, Debug, Clone)]
#[builder(pattern = "owned")]
#[builder(name = "ChatCompletionBuilder")]
#[builder(setter(strip_option, into))]
pub struct ChatCompletionRequest {
    /// ID of the model to use. Currently, only `gpt-3.5-turbo` and `gpt-3.5-turbo-0301` are supported.
    model: ModelID,
    /// The messages to generate chat completions for, in the [chat format](https://platform.openai.com/docs/guides/chat/introduction).
    messages: Vec<ChatCompletionMessage>,
    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    ///
    /// We generally recommend altering this or `top_p` but not both.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass. So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    /// We generally recommend altering this or `temperature` but not both.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    /// How many chat completion choices to generate for each input message.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u8>,
    /// If set, partial message deltas will be sent, like in ChatGPT. Tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format)
    /// as they become available, with the stream terminated by a `data: [DONE]` message.
    #[builder(setter(skip), default = "Some(true)")] // skipped until properly implemented
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// Up to 4 sequences where the API will stop generating further tokens.
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    /// The maximum number of tokens allowed for the generated answer. By default, the number of tokens the model can return will be (4096 - prompt tokens).
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far, increasing the model's likelihood to talk about new topics.
    ///
    /// [See more information about frequency and presence penalties.](https://platform.openai.com/docs/api-reference/parameter-details)
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far, decreasing the model's likelihood to repeat the same line verbatim.
    ///
    /// [See more information about frequency and presence penalties.](https://platform.openai.com/docs/api-reference/parameter-details)
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    /// Modify the likelihood of specified tokens appearing in the completion.
    ///
    /// Accepts a json object that maps tokens (specified by their token ID in the tokenizer) to an associated bias value from -100 to 100. Mathematically, the bias is added to the logits generated by the model prior to sampling. The exact effect will vary per model, but values between -1 and 1 should decrease or increase likelihood of selection; values like -100 or 100 should result in a ban or exclusive selection of the relevant token.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    logit_bias: Option<HashMap<String, f32>>,
    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices/end-user-ids).
    #[builder(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    user: String,
}

impl ChatCompletion {
    pub fn builder(
        model: ModelID,
        messages: impl Into<Vec<ChatCompletionMessage>>,
    ) -> ChatCompletionBuilder {
        ChatCompletionBuilder::create_empty()
            .model(model)
            .messages(messages)
    }

    pub async fn create(
        client: &Client,
        request: &ChatCompletionRequest,
    ) -> ApiResponseOrError<Self> {
        openai_post(client, "chat/completions", request).await
    }
}

impl ChatCompletionBuilder {
    pub async fn create(self, client: &Client) -> ApiResponseOrError<ChatCompletion> {
        ChatCompletion::create(client, &self.build().unwrap()).await
    }

    pub fn create_stream(self, client: &Client) -> impl Stream<Item = ChatCompletionEvent> + Unpin {
        let request = client
            .request(Method::POST, BASE_URL.to_owned() + "chat/completions")
            .json(&self.build().unwrap());

        let events = EventSource::new(authorization!(request)).unwrap();

        events.filter_map(|e| match e {
            Ok(Event::Message(msg)) if msg.data != "[DONE]" => {
                let x: ChatCompletionEvent = serde_json::from_str(&msg.data).unwrap();
                future::ready(Some(x))
            }
            // TODO: don't swallow all the errors here
            _ => future::ready(None),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn chat() {
        dotenv().ok();

        let chat_completion = ChatCompletion::builder(
            ModelID::Gpt3_5Turbo,
            [ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: "Hello!".to_string(),
                name: None,
            }],
        )
        .temperature(0.0)
        .create(&Client::new())
        .await
        .unwrap()
        .unwrap();

        assert_eq!(
            chat_completion.choices.first().unwrap().message.content,
            "\n\nHello there! How can I assist you today?"
        );
    }

    #[test]
    fn test_event_deserialization() {
        let role = r#"{
            "id": "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF",
            "object": "chat.completion.chunk",
            "created": 1679325191,
            "model": "gpt-3.5-turbo",
            "choices": [{"delta": {"role": "assistant"}, "index": 0, "finish_reason":null}]
        }"#;
        let content = r#"{
            "id": "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF",
            "object": "chat.completion.chunk",
            "created": 1679325191,
            "model": "gpt-3.5-turbo",
            "choices": [{"delta": {"content": "foobar"}, "index": 0, "finish_reason":null}]
        }"#;
        let end_of_stream = r#"{
            "id": "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF",
            "object": "chat.completion.chunk",
            "created": 1679325191,
            "model": "gpt-3.5-turbo",
            "choices": [{"delta": {}, "index": 0, "finish_reason": "stop"}]
        }"#;

        let role: ChatCompletionEvent = serde_json::from_str(role).unwrap();
        let content: ChatCompletionEvent = serde_json::from_str(content).unwrap();
        let end_of_stream: ChatCompletionEvent = serde_json::from_str(end_of_stream).unwrap();

        assert_eq!(
            role,
            ChatCompletionEvent {
                id: "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF".into(),
                object: "chat.completion.chunk".into(),
                created: 1679325191,
                model: ModelID::Gpt3_5Turbo,
                choices: vec![ChatCompletionChoiceDelta {
                    index: 0,
                    delta: Delta::Role {
                        role: ChatCompletionMessageRole::Assistant
                    },
                    finish_reason: None
                }]
            }
        );
        assert_eq!(
            content,
            ChatCompletionEvent {
                id: "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF".into(),
                object: "chat.completion.chunk".into(),
                created: 1679325191,
                model: ModelID::Gpt3_5Turbo,
                choices: vec![ChatCompletionChoiceDelta {
                    index: 0,
                    delta: Delta::Content {
                        content: "foobar".into()
                    },
                    finish_reason: None
                }]
            }
        );
        assert_eq!(
            end_of_stream,
            ChatCompletionEvent {
                id: "chatcmpl-6wBU7HGxEXqdShNC81ZlfkOLDM0MF".into(),
                object: "chat.completion.chunk".into(),
                created: 1679325191,
                model: ModelID::Gpt3_5Turbo,
                choices: vec![ChatCompletionChoiceDelta {
                    index: 0,
                    delta: Delta::EndOfStream {},
                    finish_reason: Some("stop".into())
                }]
            }
        );
    }
}
