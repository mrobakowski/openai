//! Get a vector representation of a given input that can be easily consumed by machine learning models and algorithms.
//!
//! Related guide: [Embeddings](https://beta.openai.com/docs/guides/embeddings)

use super::{handle_api, models::ModelID, ModifiedApiResponse, Usage};
use openai_utils::{authorization, BASE_URL};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CreateEmbeddingsRequestBody<'a> {
    model: ModelID,
    input: Vec<&'a str>,
    #[serde(skip_serializing_if = "str::is_empty")]
    user: &'a str,
}

#[derive(Deserialize)]
pub struct Embeddings {
    pub data: Vec<Embedding>,
    pub model: ModelID,
    pub usage: Usage,
}

impl Embeddings {
    /// Creates an embedding vector representing the input text.
    ///
    /// # Arguments
    ///
    /// * `model` - ID of the model to use.
    ///   You can use the [List models](https://beta.openai.com/docs/api-reference/models/list)
    ///   API to see all of your available models, or see our [Model overview](https://beta.openai.com/docs/models/overview)
    ///   for descriptions of them.
    /// * `input` - Input text to get embeddings for, encoded as a string or array of tokens.
    ///   To get embeddings for multiple inputs in a single request, pass an array of strings or array of token arrays.
    ///   Each input must not exceed 8192 tokens in length.
    /// * `user` - A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    ///   [Learn more](https://beta.openai.com/docs/guides/safety-best-practices/end-user-ids).
    pub async fn new(model: ModelID, input: Vec<&str>, user: &str) -> ModifiedApiResponse<Self> {
        let client = Client::builder().build()?;
        let request = authorization!(client.post(format!("{BASE_URL}/embeddings")))
            .json(&CreateEmbeddingsRequestBody { model, input, user });

        handle_api(request).await
    }
}

#[derive(Deserialize)]
pub struct Embedding {
    #[serde(rename = "embedding")]
    pub vec: Vec<f32>,
}

impl Embedding {
    pub async fn new(model: ModelID, input: &str, user: &str) -> ModifiedApiResponse<Self> {
        let response = Embeddings::new(model, vec![input], user).await?;

        match response {
            Ok(mut embeddings) => Ok(Ok(embeddings.data.swap_remove(0))),
            Err(error) => Ok(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn embeddings() {
        dotenv().ok();

        let embeddings = Embeddings::new(
            ModelID::TextEmbeddingAda002,
            vec!["The food was delicious and the waiter..."],
            "",
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!embeddings.data.first().unwrap().vec.is_empty())
    }

    #[tokio::test]
    async fn embedding() {
        dotenv().ok();

        let embedding = Embedding::new(
            ModelID::TextEmbeddingAda002,
            "The food was delicious and the waiter...",
            "",
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!embedding.vec.is_empty())
    }
}
