use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, Key, KeyInit,
};
use anyhow::{bail, Context, Result};
use config::{Action, Mode};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use log::{error, warn};
use state::State;
use std::collections::BTreeMap;
use std::env::args;
use std::fs::File;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use trie_rs::Trie;
use vosk::{Model, Recognizer};

use candle_transformers::models::bert::{BertModel, Config, HiddenAct, DTYPE};

use anyhow::Error as E;
use candle_core::Tensor;
use candle_nn::VarBuilder;
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::{PaddingParams, Tokenizer};

mod config;
mod llm;
mod model;
mod state;

pub struct AuthenticatedUnixStream {
    key: Key<Aes256Gcm>,
    stream: UnixStream,
}

pub struct TrieMatchBookkeeper {
    pub actions_consumed_upto: usize,
    pub trie: Trie<u8>,
    pub actions: BTreeMap<String, Action>,
    pub abstract_triggers: Trie<u8>,
    pub modes: BTreeMap<String, Mode>,
    pub modes_consumed_upto: usize,
    pub current_action: Option<Action>,
}

impl TrieMatchBookkeeper {
    fn word_to_action(
        &mut self,
        phrase: &str,
        stream: &mut Option<AuthenticatedUnixStream>,
    ) -> bool {
        if phrase.is_empty() {
            return false;
        }
        let mut start = self.actions_consumed_upto;
        for i in self.actions_consumed_upto + 2..phrase.len() + 1 {
            let search = &phrase[start..i];
            let search_results = self.trie.predictive_search(search).len();
            log::debug!("search: {:?}, results: {}", search, search_results);
            match search_results {
                0 => start = i - 1,
                1 if self.trie.exact_match(search) => {
                    self.current_action = self.actions.get(search).cloned();
                    self.do_action(stream);
                    self.actions_consumed_upto = i;
                }
                _ => {}
            }
        }

        self.actions_consumed_upto != 0
    }
    fn word_to_trigger(&mut self, phrase: &str) -> Option<Mode> {
        if phrase.is_empty() {
            return None;
        }
        let mut start = self.modes_consumed_upto;
        for i in self.modes_consumed_upto + 2..phrase.len() + 1 {
            let search = &phrase[start..i];
            let search_results = self.abstract_triggers.predictive_search(search).len();
            log::debug!("search: {:?}, results: {}", search, search_results);
            match search_results {
                0 => start = i - 1,
                1 if self.abstract_triggers.exact_match(search) => {
                    let ret = self.modes.get(search).cloned();
                    self.modes_consumed_upto = 0;
                    return ret;
                }
                _ => {}
            }
        }

        None
    }

    fn clear(&mut self) {
        self.current_action = None;
        self.actions_consumed_upto = 0;
    }
    fn do_action(&self, stream: &mut Option<AuthenticatedUnixStream>) {
        if self.current_action.is_none() {
            return;
        }
        match self.current_action.clone().unwrap() {
            Action::Keys(keys) => {
                let Some(stream) = stream else {
                    warn!("keybinding will not be executed: the daemon is not running");
                    warn!("please make sure the daemon is running as a member of the input / uinput group");
                    return;
                };
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                let cipher = Aes256Gcm::new(&stream.key);
                let ciphertext = cipher.encrypt(&nonce, keys.as_bytes()).unwrap();
                let nonce_cat_ciphertext_cat_newline =
                    format!("{}{}\n", hex::encode(nonce), hex::encode(ciphertext));
                if let Err(e) = stream
                    .stream
                    .write_all(nonce_cat_ciphertext_cat_newline.as_bytes())
                {
                    error!(
                        "failed to send keyboard shortcut `{}` to daemon socket: {}",
                        keys, e
                    );
                    error!("please make sure the daemon is running as a member of the input / uinput group");
                }
            }
            Action::Command(command) => {
                let res = if let Some((command, args)) = command.split_first() {
                    Command::new(command).args(args).spawn()
                } else if let Some(command) = command.first() {
                    Command::new(command).spawn()
                } else {
                    return;
                };
                if let Err(e) = res {
                    error!("failed to run command `{}`: {}", command[0], e);
                }
            }
        }
    }
}

fn inference_loop(
    mut stream: Option<AuthenticatedUnixStream>,
    mut state: State,
    mut bookkeeper: TrieMatchBookkeeper,
    session_rx: Receiver<ResultType>,
    mut bert: BertWithCachedKeys,
) {
    for result_type in session_rx {
        match result_type {
            ResultType::Final(sentence) => {
                log::info!("[final] {}", sentence);
                if !state.already_commanded && state.listening {
                    match bert.similarities(sentence.trim()) {
                        Err(e) => {
                            error!("failed to infer action from phrase: `{sentence}`: {e}");
                            continue;
                        }
                        Ok(Some(action_str)) => {
                            log::info!("{sentence:#?} is inferred as: {:#?}", action_str);
                            if let Some(action) = bookkeeper.actions.get(action_str) {
                                bookkeeper.current_action = Some(action.clone());
                                bookkeeper.do_action(&mut stream);
                            }
                        }
                        _ => {}
                    }
                }

                if state.infer {
                    if let Some(prompt) = sentence.get(state.length..) {
                        state
                            .to_ollama
                            .clone()
                            .expect("could not get a handle to prompt sender channel")
                            .send(prompt.to_string())
                            .expect("failed to send proompt");
                    }
                }
                state.clear();
                bookkeeper.clear();
            }
            ResultType::Partial(sentence) => {
                if sentence.len() < state.length {
                    continue;
                }
                // a bunch of indicators for sanity check
                let mode = if state.infer { "infer" } else { "eager" };
                let listening_indicator = if state.listening { "" } else { "not " };
                log::info!("[{}] [{}listening] {}", mode, listening_indicator, sentence,);

                if !state.listening && bookkeeper.word_to_trigger(&sentence) == Some(Mode::Wake) {
                    state.listening = true;
                    state.switched_modes = true;
                } else if state.listening
                    && bookkeeper.word_to_trigger(&sentence) == Some(Mode::Rest)
                {
                    state.listening = false;
                    state.switched_modes = true;
                }
                if !state.infer && state.listening && !state.switched_modes {
                    if bookkeeper.word_to_trigger(&sentence) == Some(Mode::Infer) {
                        state.infer = true;
                        continue;
                    }

                    if bookkeeper.word_to_action(&sentence, &mut stream) {
                        state.already_commanded = true;
                    }
                    state.length = sentence.len();
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let Some(audio_device) = cpal::default_host().default_input_device() else {
        bail!("no audio input device available");
    };

    let socket = match (
        UnixStream::connect("/run/tempest.socket"),
        args().into_iter().skip(1).next(),
    ) {
        (Ok(stream), Some(key)) => {
            let key_bytes = hex::decode(key.as_bytes())?;
            let key = Key::<Aes256Gcm>::from_slice(&key_bytes).clone();

            Some(AuthenticatedUnixStream { stream, key })
        }
        (Err(e), Some(_)) => {
            error!("failed to connect to the daemon socket: {e}");
            warn!("bindings to keyboard shortcuts require connection to the daemon, they will not work for this session.");
            None
        }
        _ => {
            warn!("token supplied to connect to the daemon is either nonexistent or incorrect");
            None
        }
    };

    let data_home = xdg::BaseDirectories::with_prefix("tempest")?.get_data_home();
    if !data_home.exists() {
        std::fs::create_dir(&data_home)?;
    }
    log::info!(
        "looking for model in data directory: {}",
        data_home.display()
    );

    log::info!(
        "looking for model in data directory: {}",
        data_home.display()
    );
    let model_path = data_home.join("model");
    if !model_path.exists() {
        model::download(&model_path).await?;
    }
    let model = Model::new(model_path.to_string_lossy().to_string())
        .context("failed to load vosk model into memory")?;
    let mut recognizer = Recognizer::new(&model, 16000.0).unwrap();

    recognizer.set_max_alternatives(10);
    recognizer.set_words(true);
    recognizer.set_partial_words(true);

    let conf: config::RawConfig = {
        let reader = File::open("config.yml")?;
        serde_yaml::from_reader(reader)?
    };

    let conf: config::Config = conf.into();
    let bert = BertWithCachedKeys::with_keys(conf.keys)?;

    let mut state = State::default();

    {
        let (tx, rx) = channel();
        state.ollama_channel(tx);
        let ollama = llm::Client {
            model: conf.ollama_model,
            endpoint: conf.ollama_endpoint,
            receiver: rx,
        };
        std::thread::spawn(move || ollama.handler());
    }

    let (tx, rx) = channel();
    // create an audio stream
    let maybe_stream = audio_device.build_input_stream(
        &StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        },
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            tx.send(data.to_vec())
                .expect("unable to send audio to model");
        },
        move |err| {
            log::error!("{err}");
        },
        None,
    );

    let stream = maybe_stream.context("failed to build audio input stream")?;

    let (session_tx, session_rx) = channel();

    let bookkeeper = TrieMatchBookkeeper {
        actions_consumed_upto: 0,
        modes_consumed_upto: 0,
        trie: conf.word_trie,
        abstract_triggers: conf.abstract_triggers,
        modes: conf.modes,
        actions: conf.actions,
        current_action: None,
    };

    thread::spawn(move || inference_loop(socket, state, bookkeeper, session_rx, bert));

    stream.play()?;

    let mut last_partial = String::default();
    let mut last_crib = 0;
    let mut toc = std::time::Instant::now();
    for sample in rx {
        recognizer.accept_waveform(&sample)?;
        let partial_result = recognizer.partial_result();
        let partial_text = partial_result.partial.to_string();
        let crib = partial_result.partial_result.len();
        let tic = std::time::Instant::now();
        if crib == last_crib && last_crib != 0 && tic - toc > std::time::Duration::from_secs(3) {
            let final_result = recognizer
                .final_result()
                .single()
                .map(|s| s.text.to_string())
                .unwrap_or(partial_text.clone());
            session_tx.send(ResultType::Final(final_result)).unwrap();
        } else {
            if crib != last_crib {
                last_crib = crib;
                toc = tic;
            }
            if partial_text.is_empty() {
                continue;
            }
            if partial_text == last_partial {
                continue;
            }
            session_tx
                .send(ResultType::Partial(partial_text.clone()))
                .unwrap();
        }
        last_partial = partial_text;
    }
    Ok(())
}

pub enum ResultType {
    Partial(String),
    Final(String),
}

pub struct Bert {
    model: BertModel,
    tokenizer: Tokenizer,
    device: candle_core::Device,
}
pub struct BertWithCachedKeys {
    bert: Bert,
    keys: Vec<String>,
    embeddings: Tensor,
}

impl BertWithCachedKeys {
    fn with_keys(keys: Vec<String>) -> Result<Self> {
        let mut bert = Bert::new()?;
        let embeddings = bert.cache_embeddings(keys.iter().map(|s| s.as_str()).collect())?;
        Ok(Self {
            bert,
            keys,
            embeddings,
        })
    }

    fn similarities(&mut self, sentence: &str) -> Result<Option<&str>> {
        let target = self.bert.cache_embeddings(vec![sentence])?.get(0)?;

        let mut similarities = vec![];
        for i in 0..self.keys.len() {
            let e_i = self.embeddings.get(i)?;
            let sum_ij = (&e_i * &target)?.sum_all()?.to_scalar::<f32>()?;
            let sum_i2 = (&e_i * &e_i)?.sum_all()?.to_scalar::<f32>()?;
            let sum_j2 = (&target * &target)?.sum_all()?.to_scalar::<f32>()?;
            let cosine_similarity = sum_ij / (sum_i2 * sum_j2).sqrt();
            similarities.push((cosine_similarity, i))
        }

        if let Some((similarity, index)) =
            similarities.into_iter().max_by(|u, v| u.0.total_cmp(&v.0))
        {
            println!("similarity: {similarity}");
            if similarity > 0.33 {
                return Ok(Some(self.keys[index].as_str()));
            }
        }
        Ok(None)
    }
}

impl Bert {
    fn new() -> Result<Bert> {
        let device = candle_core::Device::Cpu;
        let model_id = "sentence-transformers/all-MiniLM-L6-v2".to_string();
        let revision = "refs/pr/21".to_string();
        let repo = Repo::with_revision(model_id, RepoType::Model, revision);
        let (config_filename, tokenizer_filename, weights_filename) = {
            let api = Api::new()?;
            let api = api.repo(repo);
            let config = api.get("config.json")?;
            let tokenizer = api.get("tokenizer.json")?;
            let weights = api.get("model.safetensors")?;
            (config, tokenizer, weights)
        };
        let config = std::fs::read_to_string(config_filename)?;
        let mut config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let approximate_gelu = true;
        if approximate_gelu {
            config.hidden_act = HiddenAct::GeluApproximate;
        }
        let model = BertModel::load(vb, &config)?;
        Ok(Bert {
            model,
            tokenizer,
            device,
        })
    }
    fn cache_embeddings(&mut self, haystack: Vec<&str>) -> Result<Tensor> {
        let sentences = haystack.clone();
        if let Some(pp) = self.tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            self.tokenizer.with_padding(Some(pp));
        }
        let tokens = self
            .tokenizer
            .encode_batch(sentences.clone(), true)
            .map_err(E::msg)?;
        let token_ids = tokens
            .iter()
            .map(|tokens| {
                let tokens = tokens.get_ids().to_vec();
                Ok(Tensor::new(tokens.as_slice(), &self.device)?)
            })
            .collect::<Result<Vec<_>>>()?;

        let token_ids = Tensor::stack(&token_ids, 0)?;
        let token_type_ids = token_ids.zeros_like()?;
        println!("running inference on batch {:?}", token_ids.shape());
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        println!("generated embeddings {:?}", embeddings);
        // Apply some avg-pooling by taking the mean embedding value for all tokens (including padding)
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = normalize_l2(&embeddings)?;
        println!("pooled embeddings {:?}", embeddings.shape());
        Ok(embeddings)
    }
}

pub fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}
