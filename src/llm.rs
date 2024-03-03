use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::time::Duration;
#[derive(Serialize)]
pub struct Request {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    role: String,
    content: String,
}
#[derive(Deserialize)]
pub struct Response {
    message: Message,
}

pub fn handler(model: String, recv: Receiver<String>) {
    for prompt in recv {
        log::info!("sending to ollama: {}", prompt);
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap();
        let resp = client
            .post("http://localhost:11434/api/chat")
            .json(&Request {
                model: model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }],
                stream: false,
            })
            .send();

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("failed to send fuzzy language request to ollama: {e}");
                return;
            }
        };

        let message = match resp.json() {
            Ok(Response { message }) => message,
            Err(e) => {
                log::error!("failed to parse JSON response from ollama: {e}");
                return;
            }
        };

        log::info!("response from ollama: {}", message.content);
    }
}
