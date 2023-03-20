use openai_bootstrap::{authorization, ApiResponse, BASE_URL};
pub use openai_bootstrap::OpenAiError;
use reqwest::{Method, RequestBuilder};
use reqwest_eventsource::EventSource;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use reqwest::Client;
use futures::StreamExt;

pub mod chat;
pub mod completions;
pub mod edits;
pub mod embeddings;
pub mod models;

#[derive(Deserialize, Clone, Copy)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

type ApiResponseOrError<T> = Result<Result<T, OpenAiError>, reqwest::Error>;

async fn openai_request<F, T>(client: &Client, method: Method, route: &str, builder: F) -> ApiResponseOrError<T>
where
    F: FnOnce(RequestBuilder) -> RequestBuilder,
    T: DeserializeOwned,
{
    let mut request = client.request(method, BASE_URL.to_owned() + route);

    request = builder(request);

    let mut  events = EventSource::new(authorization!(request)).unwrap();

    while let Some(event) = events.next().await {
        dbg!(event);
    }

    // let api_response: ApiResponse<T> = todo!().send().await?.json().await?;

    todo!();

    // match api_response {
    //     ApiResponse::Ok(t) => Ok(Ok(t)),
    //     ApiResponse::Err { error } => Ok(Err(error)),
    // }
}

async fn openai_get<T>(client: &Client, route: &str) -> ApiResponseOrError<T>
where
    T: DeserializeOwned,
{
    openai_request(client, Method::GET, route, |request| request).await
}

async fn openai_post<J, T>(client: &Client, route: &str, json: &J) -> ApiResponseOrError<T>
where
    J: Serialize + ?Sized,
    T: DeserializeOwned,
{
    openai_request(client, Method::POST, route, |request| request.json(json)).await
}
