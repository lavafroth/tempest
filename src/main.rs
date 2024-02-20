use anyhow::Result;
use aprilasr::{init_april_api, Model, ResultType, Session, Token};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::sync::{self};

use std::sync::{Mutex, Once};

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
}

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens_str: Vec<String> = tokens.iter().map(|t| t.token()).collect();
    tokens_str.join("")
}

const WAKE_PHRASE: &'static str = "TEMPEST RISE";
const SLEEP_PHRASE: &'static str = "TEMPEST REST";

fn example_handler(result_type: ResultType) {
    match result_type {
        ResultType::RecognitionFinal(tokens) => {
            let sentence = tokens_to_string(tokens.unwrap());
            let mut state = TOKENS.lock().unwrap();
            if !state.already_commanded && state.listening {
                voice_command(&sentence);
            }
            if sentence.contains(WAKE_PHRASE) {
                state.listening = true;
            }
            if sentence.contains(SLEEP_PHRASE) {
                state.listening = false;
            }
            state.length = 0;
            state.position = 0;
            state.already_commanded = false;
        }
        ResultType::RecognitionPartial(tokens) => {
            let mut state = TOKENS.lock().unwrap();
            let sentence = tokens_to_string(tokens.unwrap());
            if let Some(s) = sentence.get(state.position..) {
                println!("-{s}");
                if state.listening && sentence.len() > state.length {
                    state.position = sentence.rfind(' ').unwrap_or(state.position);
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

fn voice_command(s: &str) -> bool {
    if s.contains("UP") {
        key_chord(&[KEY_LEFTMETA, KEY_DOT]);
        return true;
    }
    if s.contains("DOWN") {
        key_chord(&[KEY_LEFTMETA, KEY_COMMA]);
        return true;
    }
    if s.contains("STACK") {
        key_chord(&[KEY_LEFTMETA, KEY_I]);
        return true;
    }
    if s.contains("RELEASE") {
        key_chord(&[KEY_LEFTMETA, KEY_O]);
        return true;
    }
    if s.contains("EXIT") {
        key_chord(&[KEY_LEFTMETA, KEY_Q]);
        return true;
    }
    if s.contains("CYCLE") {
        key_chord(&[KEY_LEFTMETA, KEY_R]);
        return true;
    }
    if s.contains("CONSOLE") {
        key_chord(&[KEY_LEFTMETA, KEY_1]);
        return true;
    }
    if s.contains("QUICK SETTING") {
        key_chord(&[KEY_LEFTMETA, KEY_S]);
        return true;
    }

    return false;
}

/// Main function demonstrating basic usage of the April ASR library.
fn main() -> Result<()> {
    let device = cpal::default_host()
        .default_input_device()
        .expect("no output device available");

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
                tx.send(data.to_vec()).expect("unable to transmit audio");
            },
            move |err| {
                eprintln!("{err}");
            },
            None,
        )
        .expect("whoa big chungus");
    let session = match Session::new(model, example_handler, true, true) {
        Ok(session) => session,
        Err(_) => anyhow::bail!("session creation failed lol"),
    };
    stream.play()?;
    for bytes in rx {
        for pcm16 in bytes {
            // SAFETY: in all circumstances, an i16 can be transmuted into two bytes.
            let byte_pair = unsafe { std::mem::transmute_copy::<i16, [u8; 2]>(&pcm16) };
            session.feed_pcm16(byte_pair.to_vec());
        }
    }
    Ok(())
}
