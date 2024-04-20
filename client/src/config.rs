use serde::Deserialize;
use std::collections::BTreeMap;
use trie_rs::TrieBuilder;

#[derive(Deserialize)]
pub struct RawConfig {
    pub wake_phrase: String,
    pub rest_phrase: String,
    pub infer_phrase: String,
    pub actions: Vec<RawBinding>,
    pub ollama_model: String,
    pub ollama_endpoint: String,
}

#[derive(Clone, Debug)]
pub enum Action {
    Keys(String),
    Command(Vec<String>),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawAction {
    Keys(Vec<String>),
    Command(Vec<String>),
}

#[derive(Deserialize)]
pub struct RawBinding {
    phrase: String,

    #[serde(flatten)]
    action: RawAction,
}

pub struct Config {
    pub actions: BTreeMap<String, Action>,
    pub word_trie: trie_rs::Trie<u8>,
    pub keys: Vec<String>,
    pub abstract_triggers: trie_rs::Trie<u8>,
    pub modes: BTreeMap<String, Mode>,
    pub ollama_model: String,
    pub ollama_endpoint: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Wake,
    Rest,
    Infer,
    Custom(usize),
}

impl From<RawConfig> for Config {
    fn from(value: RawConfig) -> Self {
        let wake_phrase = value.wake_phrase.to_lowercase();
        let rest_phrase = value.rest_phrase.to_lowercase();
        let infer_phrase = value.infer_phrase.to_lowercase();
        let mut trie_builder = TrieBuilder::new();
        for phrase in value.actions.iter().map(|b| b.phrase.to_lowercase()) {
            trie_builder.push(phrase);
        }
        let word_trie = trie_builder.build();
        let keys = value
            .actions
            .iter()
            .map(|b| b.phrase.to_lowercase())
            .collect();
        let actions = value
            .actions
            .into_iter()
            .map(|b| {
                let action = match b.action {
                    RawAction::Command(v) => Action::Command(v),
                    RawAction::Keys(_) => Action::Keys(b.phrase.to_lowercase()),
                };

                (b.phrase.to_lowercase(), action)
            })
            .collect();

        let mut trie_builder = TrieBuilder::new();
        trie_builder.push(wake_phrase.clone());
        trie_builder.push(rest_phrase.clone());
        trie_builder.push(infer_phrase.clone());

        let abstract_triggers = trie_builder.build();
        let modes = [
            (wake_phrase, Mode::Wake),
            (rest_phrase, Mode::Rest),
            (infer_phrase, Mode::Infer),
        ]
        .into_iter()
        .collect();

        Self {
            abstract_triggers,
            modes,
            keys,
            actions,
            word_trie,
            ollama_model: value.ollama_model,
            ollama_endpoint: value.ollama_endpoint,
        }
    }
}
