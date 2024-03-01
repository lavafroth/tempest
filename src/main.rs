use anyhow::{anyhow, bail, Context, Result};
use aprilasr::{init_april_api, Model, ResultType, Session, Token};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use log::error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};

use std::sync::{Mutex, Once};
use std::time::Duration;

use lazy_static::lazy_static;
use mouse_keyboard_input::key_codes::*;
use mouse_keyboard_input::VirtualDevice;

mod config;

#[derive(Default)]
pub struct State {
    position: usize,
    length: usize,
    already_commanded: bool,
    listening: bool,
    infer: bool,
    to_ollama: Option<Sender<String>>,
}

/// Initialize the April API with version 1 one time only.
///
/// The function uses the `call_once` method on a static `INIT` variable. Within the closure
/// passed to `call_once`, it invokes the `init_april_api` function, passing the provided version
/// as an argument. This initialization pattern is common for scenarios where certain operations
/// need to be performed only once, such as initializing global resources.
fn initialize() {
    static INIT: Once = Once::new();
    INIT.call_once(|| init_april_api(1));
}

lazy_static! {
    static ref DEVICE: Mutex<VirtualDevice> = Mutex::new(
        VirtualDevice::default().expect("failed to create global uinput virtual device")
    );
    static ref STATE: Mutex<State> = Mutex::new(State::default());
    static ref CONFIG: Mutex<Option<config::Config>> = Mutex::new(None);
}

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens_str: Vec<String> = tokens.iter().map(|t| t.token()).collect();
    tokens_str.join("")
}

#[derive(Serialize)]
pub struct LLMRequest {
    model: String,
    messages: Vec<LLMMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LLMMessage {
    role: String,
    content: String,
}
#[derive(Deserialize)]
pub struct LLMResponse {
    message: LLMMessage,
}

fn ollama_handler(recv: Receiver<String>) {
    for prompt in recv {
        log::info!("sending to ollama: {}", prompt);
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap();
        let resp = client
            .post("http://localhost:11434/api/chat")
            .json(&LLMRequest {
                model: "mistral".to_string(),
                messages: vec![LLMMessage {
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
            Ok(LLMResponse { message }) => message,
            Err(e) => {
                log::error!("failed to parse JSON response from ollama: {e}");
                return;
            }
        };

        log::info!("response from ollama: {}", message.content);
    }
}

fn example_handler(result_type: ResultType) {
    let maybe_conf = CONFIG.lock().unwrap();
    let conf = maybe_conf.as_ref().unwrap();
    let wake_phrase: Vec<_> = conf.wake_phrase.split_whitespace().collect();
    let rest_phrase: Vec<_> = conf.rest_phrase.split_whitespace().collect();
    match result_type {
        ResultType::RecognitionFinal(tokens) => {
            let sentence = tokens_to_string(tokens.unwrap());
            let mut state = STATE.lock().unwrap();
            if !state.already_commanded && state.listening {
                voice_command(&sentence);
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
            state.length = 0;
            state.position = 0;
            state.infer = false;
            state.already_commanded = false;
        }
        ResultType::RecognitionPartial(tokens) => {
            let mut state = STATE.lock().unwrap();
            let sentence = tokens_to_string(tokens.unwrap());
            if let Some(s) = sentence.get(state.position..) {
                let mode = if state.infer { "INFER" } else { "EAGER" };
                log::info!("[{mode}]{s}");
                state.listening |= subslice_check(&sentence, &wake_phrase);
                state.listening &= !subslice_check(&sentence, &rest_phrase);
                if !state.infer && state.listening && sentence.len() > state.length {
                    state.position = sentence.rfind(' ').unwrap_or(state.position);
                    if subslice_check(s, &["LISTEN"]) {
                        state.infer = true;
                        state.position = sentence.len();
                    }
                    if voice_command(s) {
                        state.already_commanded = true;
                        state.position = sentence.len();
                    }
                    state.length = sentence.len();
                }
            }
        }
        ResultType::CantKeepUp | ResultType::Silence | ResultType::Unknown => {}
    }
}

fn key_chord(keys: &[u16]) {
    let mut device = DEVICE.lock().unwrap();
    for &key in keys {
        if let Err(e) = device.press(key) {
            log::error!("failed to press key {key}: {e}");
        }
    }
    for &key in keys.iter().rev() {
        if let Err(e) = device.release(key) {
            log::error!("failed to release key {key}: {e}");
        }
    }
}

fn subslice_check(s: &str, phrase: &[&str]) -> bool {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .windows(phrase.len())
        .any(|window| window == phrase)
}

fn key_chord_for_phrase(s: &str, phrase: &[&str], keys: &[u16]) -> bool {
    if subslice_check(s, phrase) {
        key_chord(keys);
        true
    } else {
        false
    }
}

fn command_for_phrase(s: &str, phrase: &[&str], command: &str, args: &[&str]) -> bool {
    let check = subslice_check(s, phrase);
    if check {
        if let Err(e) = Command::new(command).args(args).spawn() {
            error!("failed to run command `{}`: {}", command, e);
        }
    };
    check
}

fn voice_command(s: &str) -> bool {
    key_chord_for_phrase(s, &["UP"], &[KEY_LEFTMETA, KEY_DOT])
        || key_chord_for_phrase(s, &["DOWN"], &[KEY_LEFTMETA, KEY_COMMA])
        || key_chord_for_phrase(s, &["STACK"], &[KEY_LEFTMETA, KEY_I])
        || key_chord_for_phrase(s, &["RELEASE"], &[KEY_LEFTMETA, KEY_O])
        || key_chord_for_phrase(s, &["EXIT"], &[KEY_LEFTMETA, KEY_Q])
        || key_chord_for_phrase(s, &["CYCLE"], &[KEY_LEFTMETA, KEY_R])
        || key_chord_for_phrase(s, &["QUICK", "SETTING"], &[KEY_LEFTMETA, KEY_S])
        || key_chord_for_phrase(s, &["FILL"], &[KEY_LEFTMETA, KEY_F])
        || command_for_phrase(s, &["CONSOLE"], "blackbox", &[])
        || command_for_phrase(s, &["BROWSER"], "librewolf", &[])
        || command_for_phrase(
            s,
            &["SYSTEM", "CONFIG"],
            "xdg-open",
            &["/home/h/Public/dotfiles/configuration.nix"],
        )
        || command_for_phrase(
            s,
            &["SYSTEM", "REBUILD"],
            "pkexec",
            &[
                "doas",
                "nixos-rebuild",
                "switch",
                "--flake",
                "/home/h/Public/dotfiles#cafe",
            ],
        )
}

/// Main function demonstrating basic usage of the April ASR library.
fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let Some(audio_device) = cpal::default_host().default_input_device() else {
        bail!("no audio input device available");
    };

    initialize(); // Initialize April ASR. Required to load a Model.

    // To actually initialize the virtual device
    drop(
        DEVICE
            .lock()
            .map_err(|_| anyhow!("failed to get handle to virtual device"))?,
    );

    let conf: config::Config = {
        let reader = File::open("config.yml")?;
        serde_yaml::from_reader(reader)?
    };

    let model =
        Model::new(&conf.model_path).map_err(|e| anyhow!("failed to load april-asr model: {e}"))?;

    {
        let (tx, rx) = channel();
        STATE.lock().unwrap().to_ollama.replace(tx);
        std::thread::spawn(|| ollama_handler(rx));
    }

    CONFIG.lock().unwrap().replace(conf);

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

    // TODO: Vendor custom version of this library
    let session = match Session::new(model, example_handler, true, true) {
        Ok(session) => session,
        Err(_) => bail!("failed to create april-asr speech recognition session"),
    };
    stream.play()?;

    for bytes in rx {
        for pcm16 in bytes {
            session.feed_pcm16(pcm16.to_le_bytes().to_vec());
        }
    }
    Ok(())
}
