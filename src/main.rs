//!
//! aprilasr - rust bindings for the april-asr C api (libaprilasr)
//! Copyright (C) 2024  VHS <vhsdev@tutanota.com>
//!
//! This file is part of aprilasr.
//!
//! aprilasr is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! aprilasr is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program.  If not, see <https://www.gnu.org/licenses/>.
//!

//! # April ASR Library Example
//!
//! This example Rust file showcases basic usage of the April ASR library.
//!
//! ## Setup
//!
//! 1. Run `./makewav.sh` to create a sample English wavefile.
//! 1. Run `./getmodel.sh` to download the English April model.
//! 1. Then run `cargo run --example async` to run this file.
//!
//! ## Usage
//!
//! Run this file to see the basic functionality of the April ASR library in action.

// Import the April ASR library
use aprilasr::{init_april_api, Model, ResultType, Session, Token};

use std::io::{self, BufReader, Read};
use std::ops::Deref;
use std::sync::{Mutex, Once};
use std::time::Duration;
use std::{fmt, thread};

use lazy_static::lazy_static;
use mouse_keyboard_input::key_codes::*;
use mouse_keyboard_input::VirtualDevice;

/// April model you wish to use. Download a model using the
/// getmodel.sh shell script in the project source or using
/// any of the download links in the April ASR documentation.
const APRIL_MODEL_PATH: &str = "model.april";

// Size of read buffer for input WAV file.
const DEFAULT_BUFFER_SIZE: usize = 4096;

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
}

fn tokens_to_string(tokens: Vec<Token>) -> String {
    let tokens_str: Vec<String> = tokens.iter().map(|t| t.token()).collect();
    tokens_str.join("")
}

#[derive(Debug)]
enum WavFileError {
    IoError(io::Error),
}

impl fmt::Display for WavFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavFileError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl From<io::Error> for WavFileError {
    fn from(err: io::Error) -> Self {
        WavFileError::IoError(err)
    }
}

/// Reads data from the provided `reader` in chunks and returns the concatenated result.
///
/// This function takes a mutable reference to a type implementing the `Read` trait (`reader`),
/// reads data from it in chunks, and concatenates the chunks into a single `Vec<u8>` buffer.
///
/// # Arguments
///
/// - `reader`: A mutable reference to a type implementing the `Read` trait, providing the input data.
/// - `buffer_size`: An optional parameter specifying the size of each chunk to read. If `Some(size)` is provided,
///                  the function reads in chunks of the specified size. If `None`, it reads the entire content.
///
/// # Returns
///
/// Returns a `Result` with a `Vec<u8>` containing the concatenated data if the read operation is successful.
/// If an error occurs during the reading process, an `Err` variant is returned with an associated `io::Error`.
///
/// # Errors
///
/// This function may return an error if there is an issue reading from the provided `reader`.
/// The error type is an `io::Error` indicating the nature of the failure.
///
/// # Panics
///
/// This function may panic if there is an unexpected error during the internal memory allocation of the buffer.
/// While such panics are uncommon in normal usage, they may indicate a serious problem.
///
/// # Default Buffer Size
///
/// The default buffer size, used when `buffer_size` is not explicitly provided, is set to `DEFAULT_BUFFER_SIZE`.
/// You can adjust this constant according to your specific requirements.
fn read_wav_file<R>(reader: &mut R, buffer_size: Option<usize>) -> Result<Vec<u8>, WavFileError>
where
    R: Read,
{
    // Allocate a buffer with the specified or default size
    let buffer_size = buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE);
    let mut buffer = Vec::with_capacity(buffer_size);

    // Read as much data as available, without requiring the buffer to be completely filled
    reader.take(buffer_size as u64).read_to_end(&mut buffer)?;

    Ok(buffer)
}

fn example_handler(result_type: ResultType) {
    // dbg!(result_type.clone());
    let (prefix, tokens_str) = match result_type {
        ResultType::RecognitionFinal(tokens) => {
            let s = tokens_to_string(tokens.unwrap());
            voice_command(&s);
            ("@ ", s)
        }
        ResultType::RecognitionPartial(tokens) => ("- ", tokens_to_string(tokens.unwrap())),
        ResultType::CantKeepUp | ResultType::Silence | ResultType::Unknown => (".", String::new()),
    };

    println!("{}{}", prefix, tokens_str);
}

fn voice_command(s: &str) {
    let mut device = DEVICE.lock().unwrap();
    if s.contains("UP") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_DOT);
        device.release(KEY_DOT);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("DOWN") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_COMMA);
        device.release(KEY_COMMA);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("STACK") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_I);
        device.release(KEY_I);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("RELEASE") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_O);
        device.release(KEY_O);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("EXIT") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_Q);
        device.release(KEY_Q);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("CONSOLE") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_1);
        device.release(KEY_1);
        device.release(KEY_LEFTMETA);
    }
    if s.contains("QUICK SETTING") {
        device.press(KEY_LEFTMETA);
        device.press(KEY_S);
        device.release(KEY_S);
        device.release(KEY_LEFTMETA);
    }
}

/// Main function demonstrating basic usage of the April ASR library.
fn main() -> Result<(), io::Error> {
    initialize(); // Initialize April ASR. Required to load a Model.

    // Load an April ASR model from a file
    let model = Model::new(APRIL_MODEL_PATH).unwrap();

    // Print model metadata
    let model_sample_rate = model.sample_rate();

    {
        // To actually initialize the virtual device
        DEVICE.lock().unwrap();
    }

    println!();
    if let Ok(session) = Session::new(model, example_handler, true, true) {
        // let file = File::open(WAV_FILE_PATH)?;
        let mut buf_reader = BufReader::new(io::stdin());

        // Skip processing the wav file header
        // buf_reader.seek(SeekFrom::Start(WAV_HEADER_SIZE))?;

        // Read the WAV file in chunks until the end
        while let Some(buffer) = match read_wav_file(&mut buf_reader, Some(model_sample_rate)) {
            Ok(data) => Some(data),
            Err(err) => {
                eprintln!("Error reading WAV file: {}", err);
                None
            }
        } {
            if buffer.is_empty() {
                break; // End of file
            }

            // Feed PCM16 audio data to the session
            session.feed_pcm16(buffer);
            thread::sleep(Duration::from_millis(100));
        }

        // Flush the session after processing all data
        println!("Flushing session");
        session.flush();
    } else {
        eprintln!("Failed to create ASR session.");
    }
    Ok(())
}
