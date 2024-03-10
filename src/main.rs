use anyhow::{anyhow, bail, Context, Result};
use config::{Action, Mode};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use log::error;
use state::State;
use std::collections::BTreeMap;
use std::fs::File;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
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

pub struct TrieMatchBookkeeper {
    pub actions_consumed_upto: usize,
    pub trie: Trie<u8>,
    pub actions: BTreeMap<String, Action>,
    pub abstract_triggers: trie_rs::Trie<u8>,
    pub modes: BTreeMap<String, Mode>,
    pub modes_consumed_upto: usize,
    pub current_action: Option<Action>,
}

impl TrieMatchBookkeeper {
    fn word_to_action(&mut self, phrase: &str, vd: &mut VirtualInput) -> bool {
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
                    self.do_action(vd);
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
    fn do_action(&self, vd: &mut VirtualInput) {
        if self.current_action.is_none() {
            return;
        }
        match self.current_action.clone().unwrap() {
            Action::Keys(keys) => vd.key_chord(&keys),
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
    mut device: VirtualInput,
    mut state: State,
    mut bookkeeper: TrieMatchBookkeeper,
    session_rx: Receiver<ResultType>,
) {
    for result_type in session_rx {
        match result_type {
            ResultType::RecognitionFinal(Some(tokens)) => {
                let sentence = tokens_to_string(tokens);
                if !state.already_commanded && state.listening {
                    bookkeeper.word_to_action(&sentence, &mut device);
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
            ResultType::RecognitionPartial(Some(tokens)) => {
                let sentence = tokens_to_string(tokens);
                if sentence.len() < state.length {
                    continue;
                }
                // a bunch of indicators for sanity check
                let mode = if state.infer { "infer" } else { "eager" };
                let listening_indicator = if state.listening { "" } else { "not " };
                log::info!("[{}] [{}listening] {}", mode, listening_indicator, sentence);

                if !state.listening && bookkeeper.word_to_trigger(&sentence) == Some(Mode::Wake) {
                    state.listening = true;
                    state.already_commanded = true;
                } else if state.listening
                    && bookkeeper.word_to_trigger(&sentence) == Some(Mode::Rest)
                {
                    state.listening = false;
                    state.already_commanded = true;
                }
                if !state.infer && state.listening && !state.already_commanded {
                    if bookkeeper.word_to_trigger(&sentence) == Some(Mode::Infer) {
                        state.infer = true;
                        continue;
                    }

                    if bookkeeper.word_to_action(&sentence, &mut device) {
                        // great I dont fuckin care
                        // state.already_commanded = true;
                    }
                    state.length = sentence.len();
                }
            }
            _ => {}
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

    let mut state = State::default();

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

    let session = Session::new(&model, session_tx, false, false)
        .map_err(|e| anyhow!("failed to create april-asr speech recognition session: {e}"))?;

    let bookkeeper = TrieMatchBookkeeper {
        actions_consumed_upto: 0,
        modes_consumed_upto: 0,
        trie: conf.word_trie,
        abstract_triggers: conf.abstract_triggers,
        modes: conf.modes,
        actions: conf.actions,
        current_action: None,
    };

    thread::spawn(move || inference_loop(VirtualInput(device), state, bookkeeper, session_rx));

    stream.play()?;

    for bytes in rx {
        session.feed_pcm16(bytes);
    }
    Ok(())
}
