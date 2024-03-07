use anyhow::{anyhow, bail, Context, Result};
use config::Action;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use log::error;
use std::collections::BTreeMap;
use std::fs::File;
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread;
use tempest::{init_april_api, Model, ResultType, Session, Token};
use trie_rs::Trie;

use mouse_keyboard_input::VirtualDevice;

mod config;
mod llm;
mod state;

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens_str: Vec<String> = tokens.iter().map(|t| t.token()).collect();
    tokens_str.join("")
}

pub struct VirtualInput(VirtualDevice);

impl VirtualInput {
    fn key_chord(&mut self, keys: &[u16]) {
        for &key in keys {
            if let Err(e) = self.0.press(key) {
                error!("failed to press key {key}: {e}");
            }
        }
        for &key in keys.iter().rev() {
            if let Err(e) = self.0.release(key) {
                error!("failed to release key {key}: {e}");
            }
        }
    }
}

fn subslice_check(s: &str, phrase: &str) -> bool {
    // This does not allocate
    if s.eq_ignore_ascii_case(phrase) {
        return true;
    }
    let s = s.to_lowercase();
    let phrase = phrase.to_lowercase();
    if let Some(index) = s.find(&phrase) {
        let mut chars = s.chars();
        let before = chars.nth(index.saturating_sub(1));
        let after = chars.nth(index.saturating_add(phrase.len() + 1));
        match (before, after) {
            (Some(' ') | None, Some(' ') | None) => true,
            _ => false,
        }
    } else {
        false
    }
}

pub struct TrieMatchBookkeeper {
    pub matched: String,
    pub trie: Trie<u8>,
    pub actions: BTreeMap<String, Action>,
    pub current_action: Option<Action>,
}

impl TrieMatchBookkeeper {
    fn word_appears_in_phrases(&mut self, phrase: &str, vd: &mut VirtualInput) -> usize {
        let chars = phrase.chars();
        for c in chars {
            if self.matched.is_empty() && self.trie.predictive_search(c.to_string()).len() > 0 {
                self.matched.push(c);
            } else if !self.matched.is_empty() {
                let mut new_search = self.matched.clone();
                new_search.push(c);
                if self.trie.predictive_search(&new_search).len() > 0 {
                    // self.internal_matched_crib.replace(new_search);
                    self.matched = new_search;
                } else {
                    self.matched.clear();
                    self.matched.push(c);
                }
            }

            if !self.matched.is_empty() && self.trie.predictive_search(&self.matched).len() == 1 {
                self.current_action = self.actions.get(&self.matched).cloned();
                self.do_action(vd);
            }
            log::debug!("matched {:?}, char: {:?}", self.matched, c);
        }

        self.matched.len()
    }
    fn clear(&mut self) {
        self.current_action = None;
        self.matched.clear();
    }
    fn do_action(&self, vd: &mut VirtualInput) {
        if self.current_action.is_none() {
            return;
        }
        match self.current_action.clone().unwrap() {
            Action::Keys(keys) => vd.key_chord(&keys),
            Action::Command(command) => {
                if command.len() == 0 {
                    return;
                }
                let args = if command.len() > 1 {
                    &command[1..]
                } else {
                    &[]
                };
                if let Err(e) = Command::new(command.get(0).unwrap()).args(args).spawn() {
                    error!("failed to run command `{}`: {}", command[0], e);
                }
            }
        }
    }
}

fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let Some(audio_device) = cpal::default_host().default_input_device() else {
        bail!("no audio input device available");
    };

    init_april_api(1); // Initialize April ASR. Required to load a Model.

    let device = VirtualDevice::default()
        .map_err(|e| anyhow!("failed to create global uinput virtual device: {e}"))?;

    let conf: config::RawConfig = {
        let reader = File::open("config.yml")?;
        serde_yaml::from_reader(reader)?
    };

    let conf: config::Config = conf.into();

    let mut state = state::State::default();

    let model =
        Model::new(&conf.model_path).map_err(|e| anyhow!("failed to load april-asr model: {e}"))?;

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

    // TODO: change the callback to a channel Sender in the vendored library
    let session = Session::new(&model, session_tx, false, false)
        .map_err(|e| anyhow!("failed to create april-asr speech recognition session: {e}"))?;

    let mut bookkeeper = TrieMatchBookkeeper {
        matched: String::new(),
        trie: conf.word_trie,
        actions: conf.actions,
        current_action: None,
    };

    thread::spawn(move || {
        let mut device = VirtualInput(device);
        let wake_phrase = conf.wake_phrase;
        let rest_phrase = conf.rest_phrase;
        for result_type in session_rx {
            match result_type {
                ResultType::RecognitionFinal(Some(tokens)) => {
                    let sentence = tokens_to_string(tokens);
                    if !state.already_commanded && state.listening {
                        bookkeeper.word_appears_in_phrases(&sentence, &mut device);
                        bookkeeper.do_action(&mut device);
                    }

                    if state.infer {
                        if let Some(prompt) = sentence.get(state.position..) {
                            state
                                .to_ollama
                                .clone()
                                .expect("could not get a handle to proompt sender channel")
                                .send(prompt.to_string())
                                .expect("failed to send proompt");
                        }
                    }
                    state.clear();
                    bookkeeper.clear();
                }
                ResultType::RecognitionPartial(Some(tokens)) => {
                    let sentence = tokens_to_string(tokens);
                    if let Some(s) = sentence.get(state.position..) {
                        // a bunch of indicators for sanity check
                        let mode = if state.infer { "infer" } else { "eager" };
                        let listening_indicator = if state.listening { "" } else { "not " };
                        log::info!("[{}] [{}listening] {}", mode, listening_indicator, s);

                        if state.listening {
                            state.listening = !subslice_check(&sentence, &rest_phrase);
                            state.already_commanded = true;
                        } else {
                            state.listening = subslice_check(&sentence, &wake_phrase);
                            state.already_commanded = true;
                            continue;
                        }
                        if !state.infer && state.listening && sentence.len() > state.length {
                            state.position = sentence.rfind(' ').unwrap_or(state.position);
                            if subslice_check(s, "LISTEN") {
                                state.infer = true;
                                state.position = sentence.len();
                            }
                            let position = bookkeeper.word_appears_in_phrases(s, &mut device);
                            if position > 0 {
                                state.already_commanded = true;
                                state.position = sentence.len();
                            }
                            state.length = sentence.len();
                        }
                    }
                }
                _ => {}
            }
        }
    });

    stream.play()?;

    for bytes in rx {
        session.feed_pcm16(bytes);
    }
    Ok(())
}
