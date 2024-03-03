use anyhow::{anyhow, bail, Context, Result};
use config::Binding;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use log::error;
use std::fs::File;
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread;
use tempest::{init_april_api, Model, ResultType, Session, Token};

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
                log::error!("failed to press key {key}: {e}");
            }
        }
        for &key in keys.iter().rev() {
            if let Err(e) = self.0.release(key) {
                log::error!("failed to release key {key}: {e}");
            }
        }
    }
}

fn subslice_check(s: &str, phrase: &[&str]) -> bool {
    let phrase: Vec<String> = phrase.iter().map(|word| word.to_uppercase()).collect();
    s.split_whitespace()
        .collect::<Vec<_>>()
        .windows(phrase.len())
        .any(|window| window == phrase)
}

fn key_chord_for_phrase(vd: &mut VirtualInput, s: &str, phrase: &[&str], keys: &[u16]) -> bool {
    if subslice_check(s, phrase) {
        vd.key_chord(keys);
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

fn voice_command(bindings: &[Binding], vd: &mut VirtualInput, s: &str) -> bool {
    for binding in bindings {
        if key_chord_for_phrase(
            vd,
            s,
            &binding.phrase.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            &binding.keys,
        ) {
            return true;
        }
    }
    //     || command_for_phrase(s, &["CONSOLE"], "blackbox", &[])
    //     || command_for_phrase(s, &["BROWSER"], "librewolf", &[])
    //     || command_for_phrase(
    //         s,
    //         &["SYSTEM", "CONFIG"],
    //         "xdg-open",
    //         &["/home/h/Public/dotfiles/configuration.nix"],
    //     )
    //     || command_for_phrase(
    //         s,
    //         &["SYSTEM", "REBUILD"],
    //         "pkexec",
    //         &[
    //             "doas",
    //             "nixos-rebuild",
    //             "switch",
    //             "--flake",
    //             "/home/h/Public/dotfiles#cafe",
    //         ],
    //     )
    false
}

fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let Some(audio_device) = cpal::default_host().default_input_device() else {
        bail!("no audio input device available");
    };

    init_april_api(1); // Initialize April ASR. Required to load a Model.

    // To actually initialize the virtual device
    let device = VirtualDevice::default().expect("failed to create global uinput virtual device");

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
        std::thread::spawn(|| llm::handler(conf.ollama_model, rx));
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
    let session = match Session::new(&model, session_tx, true, false) {
        Ok(session) => session,
        Err(_) => bail!("failed to create april-asr speech recognition session"),
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
                        voice_command(&conf.bindings, &mut device, &sentence);
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
                }
                ResultType::RecognitionPartial(Some(tokens)) => {
                    let sentence = tokens_to_string(tokens);
                    if let Some(s) = sentence.get(state.position..) {
                        let mode = if state.infer { "INFER" } else { "EAGER" };
                        log::info!("[{mode}]{s}");
                        state.listening |= subslice_check(
                            &sentence,
                            &wake_phrase.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
                        );
                        state.listening &= !subslice_check(
                            &sentence,
                            &rest_phrase.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
                        );
                        if !state.infer && state.listening && sentence.len() > state.length {
                            state.position = sentence.rfind(' ').unwrap_or(state.position);
                            if subslice_check(s, &["LISTEN"]) {
                                state.infer = true;
                                state.position = sentence.len();
                            }
                            if voice_command(&conf.bindings, &mut device, s) {
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
