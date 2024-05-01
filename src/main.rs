use lambda_runtime::{
    run, service_fn,
    tracing::{self, error, info},
    Error, LambdaEvent,
};
use octocrab::models::issues::Issue;
use reqwest::{
    header::{HeaderMap, ETAG, USER_AGENT},
    Client,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct Request {
    owner: String,
    repo: String,
    issue_number: u64,
}

#[derive(Serialize, Debug)]
struct LResponse {
    etag: String,
    issue: Issue,
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<LResponse, Error> {
    info!("Event payload: {:?}", event.payload);
    let owner = event.payload.owner;
    let repo = event.payload.repo;
    let issue_number = event.payload.issue_number;

    let request_url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}");
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Issue Fetcher".parse().unwrap());

    let client = Client::new();
    let response = client.get(request_url).headers(headers).send().await?;

    if response.status().is_success() {
        let etag = response
            .headers()
            .get(ETAG)
            .ok_or_else(|| {
                error!("ETag header is missing in the response");
                Error::from("ETag header is missing in the response")
            })?
            .to_str()
            .map_err(|_| {
                error!("Failed to convert ETag header to string");
                Error::from("Failed to convert ETag header to string")
            })?
            .to_owned();

        let issue: Issue = response.json().await.map_err(|err| {
            error!("Failed to deserialize issue from response: {}", err);
            Error::from(format!(
                "Failed to deserialize issue from response: {}",
                err
            ))
        })?;

        let resp = LResponse { etag, issue };

        info!("Response: {:?}", resp);
        Ok(resp)
    } else {
        Err(Error::from("Failed to fetch issue"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}
