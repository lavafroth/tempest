use anyhow::Result;
use aprilasr::{init_april_api, Model, ResultType, Session, Token};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{self};

use std::sync::{Mutex, Once};
use std::time::Duration;

use lazy_static::lazy_static;
use mouse_keyboard_input::key_codes::*;
use mouse_keyboard_input::VirtualDevice;

// Path to the april model
// TODO: make this part of the config file
const APRIL_MODEL_PATH: &str = "model.april";

#[derive(Default)]
pub struct State {
    position: usize,
    length: usize,
    already_commanded: bool,
    listening: bool,
    infer: bool,
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
    static ref TOKENS: Mutex<State> = Mutex::new(State::default());
    // THIS IS FUCKIN RIDICULOUS
    static ref OLLAMA: Mutex<(
        Option<Sender<String>>,
        Option<Receiver<String>>
    )> = Mutex::new(create_global_channels());
}

fn create_global_channels() -> (Option<Sender<String>>, Option<Receiver<String>>) {
    let (tx, rx) = channel();
    (Some(tx), Some(rx))
}

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens_str: Vec<String> = tokens.iter().map(|t| t.token()).collect();
    tokens_str.join("")
}

const WAKE_PHRASE: &[&'static str] = &["TEMPEST", "RISE"];
const SLEEP_PHRASE: &[&'static str] = &["TEMPEST", "REST"];

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

fn call_ollama(recv: Receiver<String>) {
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
        let LLMResponse { message } = resp.unwrap().json().unwrap();
        log::info!("response from ollama: {}", message.content);
    }
}

fn example_handler(result_type: ResultType) {
    match result_type {
        ResultType::RecognitionFinal(tokens) => {
            let sentence = tokens_to_string(tokens.unwrap());
            let mut state = TOKENS.lock().unwrap();
            if !state.already_commanded && state.listening {
                voice_command(&sentence);
            }

            if state.infer {
                if let Some(prompt) = sentence.get(state.position..) {
                    OLLAMA
                        .lock()
                        .unwrap()
                        .0
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
            let mut state = TOKENS.lock().unwrap();
            let sentence = tokens_to_string(tokens.unwrap());
            if let Some(s) = sentence.get(state.position..) {
                println!("-{s}");
                state.listening |= subslice_check(&sentence, WAKE_PHRASE);
                state.listening &= !subslice_check(&sentence, SLEEP_PHRASE);
                if !state.infer && state.listening && sentence.len() > state.length {
                    state.position = sentence.rfind(' ').unwrap_or(state.position);
                    if subslice_check(s, &["LISTEN"]) {
                        state.infer = true;
                    }
                    if voice_command(&s) {
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
        device
            .press(key)
            .expect("failed to press key through uinput");
    }
    for &key in keys.iter().rev() {
        device
            .release(key)
            .expect("failed to release key through uinput");
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

fn voice_command(s: &str) -> bool {
    key_chord_for_phrase(s, &["UP"], &[KEY_LEFTMETA, KEY_DOT])
        || key_chord_for_phrase(s, &["DOWN"], &[KEY_LEFTMETA, KEY_COMMA])
        || key_chord_for_phrase(s, &["STACK"], &[KEY_LEFTMETA, KEY_I])
        || key_chord_for_phrase(s, &["RELEASE"], &[KEY_LEFTMETA, KEY_O])
        || key_chord_for_phrase(s, &["EXIT"], &[KEY_LEFTMETA, KEY_Q])
        || key_chord_for_phrase(s, &["CYCLE"], &[KEY_LEFTMETA, KEY_R])
        || key_chord_for_phrase(s, &["CONSOLE"], &[KEY_LEFTMETA, KEY_1])
        || key_chord_for_phrase(s, &["QUICK", "SETTING"], &[KEY_LEFTMETA, KEY_S])
}

/// Main function demonstrating basic usage of the April ASR library.
fn main() -> Result<()> {
    let device = cpal::default_host()
        .default_input_device()
        .expect("no output device available");

    simple_logger::init_with_level(log::Level::Info)?;

    initialize(); // Initialize April ASR. Required to load a Model.

    // To actually initialize the virtual device
    drop(
        DEVICE
            .lock()
            .expect("failed to get handle to virtual device"),
    );
    let model = Model::new(APRIL_MODEL_PATH).unwrap();

    let (tx, rx) = sync::mpsc::channel();

    // Flush the session after processing all data
    let stream = device
        .build_input_stream(
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
                eprintln!("{err}");
            },
            None,
        )
        .expect("whoa big chungus");
    let session = match Session::new(model, example_handler, true, true) {
        Ok(session) => session,
        Err(_) => anyhow::bail!("session creation failed"),
    };
    stream.play()?;

    // God I fucking hate doing this with channels and mutexes.
    let recv = OLLAMA.lock().unwrap().1.take().unwrap();
    std::thread::spawn(|| call_ollama(recv));
    for bytes in rx {
        for pcm16 in bytes {
            // SAFETY: in all circumstances, an i16 can be transmuted into two bytes.
            let byte_pair = unsafe { std::mem::transmute_copy::<i16, [u8; 2]>(&pcm16) };
            session.feed_pcm16(byte_pair.to_vec());
        }
    }
    Ok(())
}
