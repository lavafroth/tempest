use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub model_path: String,
    pub wake_phrase: String,
    pub rest_phrase: String,
}
