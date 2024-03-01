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

//! This module provides a Rust interface for interacting with the April ASR library,
//! allowing developers to leverage speech-to-text capabilities in Rust applications.
use aprilasr_sys::ffi as afi;
use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::Sender;
use std::{default, fmt, process, slice};

/// Exposes the April API version as defined by the FFI cast to `i32`.
pub static APRIL_VERSION: i32 = afi::APRIL_VERSION as c_int;

/// Initializes the April API.
///
/// Must be called once before creating a Model.
///
/// Pass APRIL_VERSION as argument like so: aam_api_init(APRIL_VERSION).
///
/// # Safety
/// This function should be called in a safe, single-threaded context to initialize the April API safely.
///
/// # Panics
/// - Panics when the provided version is unexpected.
pub fn init_april_api(version: i32) {
    match version {
        1 => unsafe { afi::aam_api_init(version) },
        _ => panic!("Unsupported version. Wanted: 1. Got: {:?}", version),
    };
}

/// Wrapper for managing an April ASR model running in memory.
///
/// This struct provides a safe Rust interface for interacting with the April ASR model.
/// It is responsible for managing the lifecycle of the underlying model, including creation,
/// retrieval of information (name, description, language, sample rate), and destruction.
///
/// # Safety
/// The `Model` struct implements the [`Drop`] trait to ensure proper resource cleanup.
///
/// The implementation of the `Drop` trait guarantees that resources associated with the
/// April ASR model are released correctly when a `Model` instance goes out of scope.
///
/// Users should ensure that all instances of `Model` are properly managed and that no
/// references to the model are held beyond their intended lifespan to prevent resource leaks.
///
/// # Examples
///
/// Example usage of the `Model` struct can be found in the module's documentation.
///
/// [`Drop`]: std::ops::Drop
#[derive(Debug)]
pub struct Model {
    ctx: *mut afi::AprilASRModel_i,
}

impl Model {
    /// Instantiate an April ASR model given a file path.
    ///
    /// # Arguments
    ///
    /// * `model_path` - The file path to the April ASR model.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be created from the provided file path.
    pub fn new(model_path: &str) -> Result<Model, Box<dyn std::error::Error>> {
        let path = CString::new(model_path).expect("CString::new failed");
        let model = unsafe { afi::aam_create_model(path.as_ptr()) };

        if model.is_null() {
            Err("Failed to create ASR model".into())
        } else {
            Ok(Model { ctx: model })
        }
    }

    /// Get the name of the model.
    ///
    /// # Safety
    /// Guarantees a freshly-owned `String` allocation.
    pub fn name(&self) -> String {
        let cstr = unsafe { CStr::from_ptr(afi::aam_get_name(self.ctx)) };
        String::from_utf8_lossy(cstr.to_bytes()).to_string()
    }

    /// Get the description of the model.
    ///
    /// # Safety
    /// Guarantees a freshly-owned `String` allocation.
    pub fn description(&self) -> String {
        let cstr = unsafe { CStr::from_ptr(afi::aam_get_description(self.ctx)) };
        String::from_utf8_lossy(cstr.to_bytes()).to_string()
    }

    /// Get the language of the model.
    ///
    /// # Safety
    /// Guarantees a freshly-owned `String` allocation.
    pub fn language(&self) -> String {
        let cstr = unsafe { CStr::from_ptr(afi::aam_get_language(self.ctx)) };
        String::from_utf8_lossy(cstr.to_bytes()).to_string()
    }

    /// Get the sample rate of the model.
    pub fn sample_rate(&self) -> usize {
        unsafe { afi::aam_get_sample_rate(self.ctx) }
    }
}

/// Implementation of the `Drop` trait for the `Model` struct.
///
/// The `Drop` trait defines a method named `drop` that is called when the value
/// goes out of scope. In this implementation, it is used to release the resources
/// associated with the April ASR model, ensuring proper cleanup.
///
/// # Safety
///
/// The `afi::aam_free` function is marked as `unsafe` because it deals with raw
/// pointers and memory management. The implementation assumes that the
/// `aprilasr_sys` crate provides a safe and correct way to free the resources
/// associated with the ASR model. Incorrect usage of this function or invalid
/// pointers may result in undefined behavior.
impl Drop for Model {
    /// Drops the April ASR model, releasing associated resources.
    fn drop(&mut self) {
        unsafe { afi::aam_free(self.ctx) }
    }
}

/// Represents flag bits associated with speech recognition result tokens.
///
/// This enum provides information about specific characteristics associated with
/// speech recognition result tokens. It is used to mark the start of a new word
/// or the end of a sentence in the recognized text.
///
/// # Variants
///
/// - `WordBoundary`: If set, this token marks the start of a new word.
///
/// - `SentenceEnd`: If set, this token marks the end of a sentence, meaning the token
///   is equal to ".", "!", or "?". Some models may not have this token.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenFlagBits {
    /// Undocumented feature. This is not found in the flag
    /// bits returned by the Bindgen bindings but it happens.
    Zero,

    /// If set, this token marks the start of a new word.
    // In English, this is equivalent to (token[0] == ' ').
    WordBoundary,

    /// If set, this token marks the end of a sentence, meaning the token is
    /// equal to ".", "!", or "?". Some models may not have this token.
    SentenceEnd,
}

impl From<afi::AprilTokenFlagBits> for TokenFlagBits {
    /// Converts from the FFI representation to the Rust enum.
    ///
    /// # Panics
    ///
    /// Panics if an invalid FFI flag bit value is encountered.
    fn from(flag_bit: afi::AprilTokenFlagBits) -> Self {
        match flag_bit {
            0 => TokenFlagBits::Zero,
            afi::AprilTokenFlagBits_APRIL_TOKEN_FLAG_WORD_BOUNDARY_BIT => {
                TokenFlagBits::WordBoundary
            }
            afi::AprilTokenFlagBits_APRIL_TOKEN_FLAG_SENTENCE_END_BIT => TokenFlagBits::SentenceEnd,
            _ => unreachable!("Unexpected AprilTokenFlagBits"),
        }
    }
}

/// Unique identifier for a speaker.
///
/// This struct represents a unique identifier for a speaker, which can be used as a
/// hash of the speaker's name or other distinguishing characteristics. The identifier
/// can be provided to [`aas_create_session`](fn.aas_create_session.html) for saving and
/// restoring state associated with the speaker. If the identifier is set to all zeros,
/// it will be ignored.
///
/// Please note that as of now, the functionality related to `SpeakerID` is not
/// fully implemented, and setting or using the speaker ID may have no effect.
///
/// # Example
///
/// ```rust
/// use aprilasr::SpeakerID;
///
/// // Create a new SpeakerID
/// let speaker_id = SpeakerID {
///     data: [0; 16], // All zeros (ignored in the current implementation)
/// };
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct SpeakerID {
    pub data: [u8; 16usize],
}

/// Implements the `Default` trait for `SpeakerID`.
///
/// The `Default` implementation creates a `SpeakerID` instance with all zeros in the `data` field.
impl Default for SpeakerID {
    fn default() -> Self {
        Self { data: [0; 16] }
    }
}

/// Provides a conversion from low-level FFI bindings [`afi::AprilSpeakerID`] to [`SpeakerID`].
///
/// # Safety
///
/// The function assumes that the provided [`afi::AprilSpeakerID`] is valid and properly initialized.
/// Incorrect or uninitialized values may lead to undefined behavior.
impl From<afi::AprilSpeakerID> for SpeakerID {
    fn from(t: afi::AprilSpeakerID) -> Self {
        // Assuming a straightforward conversion is possible
        SpeakerID { data: t.data }
    }
}

/// Implements the `Into` trait for converting `SpeakerID` into the low-level FFI representation `afi::AprilSpeakerID`.
///
/// This `Into` implementation allows seamless conversion of a Rust-friendly `SpeakerID` into the corresponding
/// low-level FFI representation used by `afi::AprilSpeakerID`.
impl Into<afi::AprilSpeakerID> for SpeakerID {
    fn into(self) -> afi::AprilSpeakerID {
        afi::AprilSpeakerID { data: self.data }
    }
}

// Enumeration of April recognition result types.
///
/// This enum represents different types of recognition results that can be returned by
/// the April library. Each variant provides information about the nature of the recognition
/// result, such as whether it is unknown, partial, final, or indicates an error condition.
///
/// ## Variants
///
/// - `Unknown`: Specifies that the result is unknown.
///
/// - `RecognitionPartial`: Specifies that the result is only partial, and a future call will contain
///   much of the same text but updated. Contains a vector of [`Token`] instances representing
///   the recognized text tokens.
///
/// - `RecognitionFinal`: Specifies that the result is final. Future calls will start from
///   empty and will not contain any of the given text. Contains a vector of [`Token`] instances
///   representing the recognized text tokens.
///
/// - `CantKeepUp`: If in non-synchronous mode, this may be called when the internal audio
///   buffer is full and processing can't keep up. It will be called with count = 0, tokens = `None`.
///
/// - `Silence`: Specifies that there has been some silence. Will not be called repeatedly.
///   It will be called with count = 0, tokens = `None`.
///
/// [`Token`]: enum.Token.html
#[derive(Debug, Clone)]
#[repr(i32)]
pub enum ResultType {
    /// Specifies that the result is unknown.
    Unknown,

    /// Specifies that the result is only partial, and a future call will
    /// contain much of the same text but updated.
    RecognitionPartial(Option<Vec<Token>>),

    /// Specifies that the result is final. Future calls will start from
    /// empty and will not contain any of the given text.
    RecognitionFinal(Option<Vec<Token>>),

    /// If in non-synchronous mode, this may be called when the internal
    /// audio buffer is full and processing can't keep up.
    /// It will be called with count = 0, tokens = `None`.
    CantKeepUp,

    /// Specifies that there has been some silence. Will not be called
    /// repeatedly.
    /// It will be called with count = 0, tokens = `None`.
    Silence,
}

/// Custom error type for errors during Token construction.
///
/// This error type is used to represent errors that may occur during the instantiation
/// of [`Token`](struct.Token.html). It provides additional information about
/// the nature of the error, such as an invalid flag value in the FFI bindings.
///
/// # Example
///
/// ```rust
/// use aprilasr::{Token, TokenError, TokenFlagBits};
///
/// fn create_april_token() -> Result<Token, Box<dyn std::error::Error>> {
///     // Some code that may result in an error during Token creation
///     // ...
///
///     // For simplicity, assume an error condition occurs
///     Err(Box::new(TokenError {
///         message: "Invalid TokenFlagBits value!",
///     }))
/// }
/// ```
#[derive(Debug)]
pub struct TokenError {
    pub message: &'static str,
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TokenError {}

/// Represents a speech recognition result token.
///
/// This struct encapsulates information about a speech recognition result token,
/// including the recognized text, log probability, associated flags, and the timestamp
/// at which it was emitted.
#[derive(Debug, Clone)]
pub struct Token {
    token: String,
    logprob: c_float,
    flags: TokenFlagBits,
    time_ms: usize,
}

impl Token {
    /// Instantiates a speech recognition result token.
    ///
    /// # Safety
    ///
    /// This function assumes that the provided `token` pointer is valid and points to a
    /// null-terminated C string. It also assumes that the `flags` parameter is a valid
    /// representation of `afi::AprilTokenFlagBits`. The user should ensure that the input
    /// parameters adhere to these assumptions to prevent undefined behavior.
    ///
    /// The `token` parameter is a C string pointer representing the recognition result token.
    ///
    /// The `logprob` parameter is the log probability of this token being the correct token.
    ///
    /// The `flags` parameter represents the flag bits associated with the token,
    /// and it should be a valid variant of [`TokenFlagBits`](enum.TokenFlagBits.html).
    ///
    /// The `time_ms` parameter denotes the millisecond at which this token was emitted.
    ///
    /// # Returns
    ///
    /// Returns a result containing a newly constructed `Token` instance if the
    /// instantiation is successful. If there are errors during the instantiation,
    /// such as invalid flag values, it returns a boxed error implementing the `Error` trait.
    pub fn new(
        token: *const c_char,
        logprob: c_float,
        flags: afi::AprilTokenFlagBits,
        time_ms: usize,
    ) -> Result<Token, Box<dyn std::error::Error>> {
        let token_cstr = unsafe { CStr::from_ptr(token) };
        let rust_token = String::from_utf8_lossy(token_cstr.to_bytes()).to_string();
        let rust_flags = TokenFlagBits::from(flags);

        Ok(Token {
            token: rust_token,
            logprob,
            flags: rust_flags,
            time_ms,
        })
    }

    /// Returns the recognition result token.
    ///
    /// The returned string contains its own formatting, which may denote the start of
    /// a new word or the next part of a word.
    pub fn token(&self) -> String {
        self.token.clone()
    }

    /// Returns the log probability of this being the correct token.
    pub fn logprob(&self) -> f32 {
        self.logprob
    }

    /// Returns the flag bits associated with the token.
    ///
    /// See [`TokenFlagBits`](enum.TokenFlagBits.html) for possible values.
    pub fn flags(&self) -> TokenFlagBits {
        self.flags
    }

    /// Returns the millisecond at which this token was emitted.
    ///
    /// The counting is based on how much audio is being fed, and time is not advanced
    /// when the session is not given audio.
    pub fn time_ms(&self) -> usize {
        self.time_ms
    }
}

/// Provides a conversion from low-level FFI bindings [`afi::AprilToken`] to [`Token`].
///
/// This implementation allows for more ergonomic conversion directly from low-level FFI bindings.
/// Bear in mind that implementing the `From` trait automatically provides the `Into` trait.
impl From<afi::AprilToken> for Token {
    fn from(t: afi::AprilToken) -> Self {
        Token::new(t.token, t.logprob, t.flags, t.time_ms)
            .unwrap_or_else(|err| panic!("Failed to create Token: {}", err))
    }
}

/// Enumeration of April configuration flags.
///
/// This enum represents various configuration flags that can be used with the April library.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(i32)]
pub enum ConfigFlagBits {
    /// Represents the zero bit.
    Zero,

    /// If set, the input audio should be fed in real-time (1 second of audio per second) in small chunks.
    /// Calls to `aas_feed_pcm16` and `aas_flush` will be fast as it will delegate processing to a background thread.
    /// The handler will be called from the background thread at some point later.
    /// The accuracy may be degraded depending on the system hardware.
    /// You may get an accuracy estimate by calling `aas_realtime_get_speedup`.
    #[default]
    AsyncNoRealtime,

    /// Similar to `AsyncNoRealtime`, but does not degrade accuracy depending on system hardware.
    /// However, if the system is not fast enough to process audio, the background thread will fall behind,
    /// results may become unusable, and the handler will be called with `APRIL_RESULT_ERROR_CANT_KEEP_UP`.
    AsyncRealtime,
}

/// Provides a conversion from low-level FFI bindings [`afi::AprilConfigFlagBits`] to [`ConfigFlagBits`].
///
/// # Safety
///
/// The function assumes that the provided [`afi::AprilConfigFlagBits`] is a valid variant of the enum.
/// Incorrect or invalid values may lead to undefined behavior.
impl From<afi::AprilConfigFlagBits> for ConfigFlagBits {
    fn from(bit: afi::AprilConfigFlagBits) -> Self {
        match bit {
            afi::AprilConfigFlagBits_APRIL_CONFIG_FLAG_ASYNC_NO_RT_BIT => {
                ConfigFlagBits::AsyncNoRealtime
            }
            afi::AprilConfigFlagBits_APRIL_CONFIG_FLAG_ASYNC_RT_BIT => {
                ConfigFlagBits::AsyncRealtime
            }
            afi::AprilConfigFlagBits_APRIL_CONFIG_FLAG_ZERO_BIT => ConfigFlagBits::Zero,
            _ => unreachable!("Unexpected AprilConfigFlagBits"),
        }
    }
}

/// Implements the `Into` trait for converting `ConfigFlagBits` into the low-level FFI representation `afi::AprilConfigFlagBits`.
///
/// This `Into` implementation enables seamless conversion from the Rust-friendly `ConfigFlagBits` enum
/// to its corresponding low-level FFI representation used by `afi::AprilConfigFlagBits`.
impl Into<afi::AprilConfigFlagBits> for ConfigFlagBits {
    /// Converts a `ConfigFlagBits` enum into its low-level FFI representation (`afi::AprilConfigFlagBits`).
    ///
    /// # Arguments
    ///
    /// * `self` - The Rust-friendly `ConfigFlagBits` enum to be converted.
    ///
    /// # Returns
    ///
    /// The low-level FFI representation of `ConfigFlagBits` as `afi::AprilConfigFlagBits`.
    fn into(self) -> afi::AprilConfigFlagBits {
        self as afi::AprilConfigFlagBits
    }
}

/// Configuration for the April ASR system.
///
/// This struct encapsulates the configuration parameters for the April ASR system,
/// providing a flexible setup for customization. It includes the speaker identifier,
/// recognition result handler, user data, and configuration flags.
///
/// # Fields
///
/// - `speaker`: Unique identifier for the speaker. This can be utilized as a hash
///   of the speaker's name or other distinguishing characteristics. It is used in
///   conjunction with [`aas_create_session`](fn.aas_create_session.html) for saving
///   and restoring state associated with the speaker.
///
/// - `handler`: The handler that will be called as recognition events occur. This
///   may be invoked from a different thread, so appropriate synchronization mechanisms
///   should be employed if necessary.
///
/// - `userdata`: A pointer to user-specific data that can be associated with the
///   configuration. This data is passed along to the recognition result handler,
///   allowing users to pass additional information as needed.
///
/// - `flags`: Configuration flags represented by [`ConfigFlagBits`]. These flags
///   provide options for adjusting the behavior of the ASR system, such as enabling
///   real-time processing or specifying how asynchronous processing should be handled.
///
/// # Safety
///
/// Creating a `Config` instance assumes that the provided values in the `afi::AprilConfig` are valid
/// and properly initialized. Incorrect or uninitialized values may lead to undefined behavior.
#[derive(PartialEq, Debug)]
pub struct Config {
    speaker: SpeakerID,

    /// The handler that will be called as events occur. This may be called from a different thread.
    handler: afi::AprilRecognitionResultHandler,
    userdata: *mut ::std::os::raw::c_void,

    /// See [`ConfigFlagBits`].
    flags: ConfigFlagBits,
}

impl Config {
    /// Creates a new configuration with the provided parameters.
    ///
    /// # Arguments
    ///
    /// - `speaker`: Unique identifier for the speaker.
    /// - `handler`: The handler to be called on recognition events.
    /// - `userdata`: User-specific data associated with the configuration.
    /// - `flags`: Configuration flags.
    ///
    /// # Returns
    ///
    /// Result containing a new `Config` instance with the specified parameters,
    /// or an error if any of the required parameters are missing.
    pub fn new(
        speaker: SpeakerID,
        handler: afi::AprilRecognitionResultHandler,
        userdata: *mut ::std::os::raw::c_void,
        flags: ConfigFlagBits,
    ) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(Config {
            speaker,
            handler,
            userdata,
            flags,
        })
    }

    /// Gets the speaker identifier.
    ///
    /// # Returns
    ///
    /// A reference to the speaker identifier.
    pub fn speaker(&self) -> &SpeakerID {
        &self.speaker
    }

    /// Gets the recognition result handler.
    ///
    /// # Returns
    ///
    /// The recognition result handler.
    pub fn handler(&self) -> afi::AprilRecognitionResultHandler {
        self.handler
    }

    /// Gets the user data.
    ///
    /// # Returns
    ///
    /// A pointer to the user data.
    pub fn userdata(&self) -> *mut ::std::os::raw::c_void {
        self.userdata
    }

    /// Gets the configuration flags.
    ///
    /// # Returns
    ///
    /// The configuration flags.
    pub fn flags(&self) -> ConfigFlagBits {
        self.flags
    }
}

/// Conversion from low-level FFI representation (`afi::AprilConfig`) to the Rust-friendly `Config`.
///
/// This implementation enables the creation of a `Config` instance based on the low-level FFI representation
/// provided by the `afi::AprilConfig` type.
///
/// # Safety
///
/// This function assumes that the incoming `afi::AprilConfig` value is valid and properly initialized.
/// Using incorrect or uninitialized values may result in undefined behavior.
impl From<afi::AprilConfig> for Config {
    /// Converts a `afi::AprilConfig` into a `Config` instance.
    ///
    /// # Arguments
    ///
    /// * `cfg` - The low-level FFI representation of `Config` to be converted.
    ///
    /// # Panics
    ///
    /// Panics if the creation of `Config` fails. This typically occurs when the provided
    /// `afi::AprilConfig` values result in an invalid configuration.
    fn from(cfg: afi::AprilConfig) -> Self {
        // Extract values from the FFI representation and convert them into the corresponding Rust types
        let speaker = SpeakerID::from(cfg.speaker);
        let handler = cfg.handler;
        let userdata = cfg.userdata;
        let flags = ConfigFlagBits::from(cfg.flags);

        // Attempt to create a new Config instance, panicking if the creation fails
        Config::new(speaker, handler, userdata, flags)
            .unwrap_or_else(|err| panic!("Failed to create Config: {}", err))
    }
}

/// Conversion from the Rust-friendly `Config` to the low-level FFI representation (`afi::AprilConfig`).
///
/// This implementation enables the creation of a `afi::AprilConfig` instance based on the Rust-friendly `Config`.
impl Into<afi::AprilConfig> for Config {
    /// Converts a `Config` into a `afi::AprilConfig` instance.
    ///
    /// # Arguments
    ///
    /// * `config` - The Rust-friendly `Config` to be converted into low-level FFI representation.
    fn into(self) -> afi::AprilConfig {
        // Convert Rust types into the corresponding FFI representation
        let speaker = self.speaker.into();
        let handler = self.handler;
        let userdata = self.userdata;
        let flags = self.flags.into();

        // Create a new afi::AprilConfig instance
        afi::AprilConfig {
            speaker,
            handler,
            userdata,
            flags,
        }
    }
}

/// Builder for creating and customizing `Config` instances with default or specific parameters.
///
/// This builder pattern is designed to offer ergonomic and efficient configuration creation
/// by using mutable references.
#[derive(Default)]
pub struct ConfigBuilder {
    speaker: Option<SpeakerID>,
    handler: Option<afi::AprilRecognitionResultHandler>,
    userdata: Option<*mut ::std::os::raw::c_void>,
    flags: ConfigFlagBits,
}

impl ConfigBuilder {
    /// Creates a new `ConfigBuilder` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the speaker ID.
    pub fn speaker(&mut self, speaker: SpeakerID) -> &mut Self {
        self.speaker = Some(speaker);
        self
    }

    /// Sets the recognition result handler.
    pub fn handler(&mut self, handler: afi::AprilRecognitionResultHandler) -> &mut Self {
        self.handler = Some(handler);
        self
    }

    /// Sets the user-specific data.
    pub fn userdata(&mut self, userdata: *mut ::std::os::raw::c_void) -> &mut Self {
        self.userdata = Some(userdata);
        self
    }

    /// Sets the configuration flags.
    pub fn flags(&mut self, flags: ConfigFlagBits) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Builds the `Config` instance.
    pub fn build(&self) -> Result<Config, Box<dyn std::error::Error>> {
        let speaker = self.speaker.ok_or("Speaker ID not set")?;
        let handler = self.handler.ok_or("Recognition result handler not set")?;
        let userdata = self.userdata.ok_or("User-specific data not set")?;
        let flags = self.flags;

        Ok(Config {
            speaker,
            handler,
            userdata,
            flags,
        })
    }
}

/// Wrapper function for the C callback `handler_cb_wrapper`.
///
/// This function is intended to be used as a callback from a C API. It translates the C-style
/// callback parameters into Rust types, performs some logic based on the result type, and then
/// invokes a user-provided Rust callback function.
///
/// # Safety
///
/// The callback function is marked as `unsafe` due to the transmutation of the `userdata` pointer
/// and the use of `unsafe` code to work with raw pointers. Users of this function should ensure
/// that the callback function adheres to the expected signature (`fn(ResultType) -> ()`) and
/// that the `userdata` pointer is valid and points to a valid callback function.
///
/// # Arguments
///
/// * `userdata`: A pointer to user-specific data or a callback function.
/// * `result_type`: The result type received from the C API.
/// * `count`: The number of tokens in the `tokens` array.
/// * `tokens`: A pointer to an array of `afi::AprilToken` elements.
///
/// # Panics
///
/// If the closure passed to `catch_unwind` panics, this function prints the panic information
/// to the standard error stream and aborts the process.
pub extern "C" fn handler_cb_wrapper(
    userdata: *mut std::os::raw::c_void,
    result_type: afi::AprilResultType,
    count: usize,
    tokens: *const afi::AprilToken,
) {
    if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
        let result: ResultType = match result_type {
            afi::AprilResultType_APRIL_RESULT_UNKNOWN => ResultType::Unknown,
            afi::AprilResultType_APRIL_RESULT_RECOGNITION_PARTIAL
            | afi::AprilResultType_APRIL_RESULT_RECOGNITION_FINAL => {
                let tokens_slice = unsafe { slice::from_raw_parts(tokens, count) };
                let tokens_vec: Vec<Token> = tokens_slice.iter().map(|t| (*t).into()).collect();
                if result_type == afi::AprilResultType_APRIL_RESULT_RECOGNITION_PARTIAL {
                    ResultType::RecognitionPartial(Some(tokens_vec))
                } else {
                    ResultType::RecognitionFinal(Some(tokens_vec))
                }
            }
            afi::AprilResultType_APRIL_RESULT_ERROR_CANT_KEEP_UP => ResultType::CantKeepUp,
            afi::AprilResultType_APRIL_RESULT_SILENCE => ResultType::Silence,
            _ => unreachable!("Unexpected AprilResultType"),
        };

        unsafe {
            let userdata: *mut Sender<ResultType> = std::mem::transmute(userdata);
            userdata
                .as_mut()
                .expect("unable to get a handle to the april result sender")
                .send(result)
                .expect("failed to send april result sender through the supplied channel");
        }
    })) {
        eprintln!("{:?}", e);
        process::abort();
    }
}

/// Wrapper for managing an April ASR session running in memory.
///
/// The `Session` struct encapsulates the functionality of an ASR session and provides methods for interacting with it.
/// It is responsible for managing the session's lifecycle, including creation and automatic resource cleanup upon dropping.
///
/// # Safety
///
/// The `Session` struct is marked as `unsafe` because it encapsulates low-level operations, and misuse can lead to
/// undefined behavior. It implements the [`Drop`] trait to ensure proper resource cleanup when a `Session` instance goes out of scope.
///
/// The `Session` holds a raw pointer `ctx` to the underlying April ASR session, and it is the responsibility of the user
/// to ensure that the associated resources are properly managed and that the session is used safely within the constraints
/// of the April ASR library.
///
/// The `Session` also holds a reference to the `Model` using a simple reference (`&'a Model`). This ensures that the `Model`
/// is not deallocated before the associated `Session` instances are closed. The ownership and lifecycle management of the `Model`
/// are abstracted away, providing a safe way to share the model among multiple sessions.
///
/// Users should ensure that all instances of `Session` are properly managed and that no
/// references to the session are held beyond their intended lifespan to prevent resource leaks.
///
/// # Examples
///
/// Example usage of the `Session` struct can be found in the module's documentation.
///
/// [`Drop`]: std::ops::Drop
#[derive(Debug)]
pub struct Session<'a> {
    ctx: *mut afi::AprilASRSession_i,
    // Hold onto the model reference
    _model: &'a Model,
}

impl<'a> Session<'a> {
    /// Initializes a new ASR session with the specified ASR model and configuration.
    ///
    /// # Safety
    ///
    /// The safety of this function relies on the correctness of the underlying FFI library's
    /// `aas_create_session` function. Incorrect usage or invalid parameters may result in
    /// undefined behavior.
    ///
    /// Additionally, this function internally uses [`Arc`] (Atomic Reference Counting) to ensure that
    /// the provided [`Model`] is not dropped before the associated `Session` instances are closed.
    /// This guarantees that the `Model` remains valid for the duration of the `Session`.
    ///
    /// # Arguments
    ///
    /// * `model` - The [`Model`] to be used for the session.
    /// * `callback` - A callback function to handle the result of the ASR session asynchronously.
    /// * `asynchronous` - A flag indicating whether the ASR session should run asynchronously.
    /// * `no_rt` - A flag indicating whether real-time processing should be disabled.
    /// * `speaker_name` - (Currently commented out) The name of the speaker associated with the session.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing either the newly created `Session` instance or an error message.
    ///
    /// # Examples
    ///
    /// Example usage of the `new` function can be found in the module's documentation.
    ///
    /// [`Arc`]: std::sync::Arc
    /// [`Model`]: struct.Model.html
    pub fn new(
        model: &'a Model,
        callback: Sender<ResultType>,
        asynchronous: bool,
        no_rt: bool,
        // speaker_name: &str,
    ) -> Result<Session, Box<dyn std::error::Error>> {
        let mut config_builder = ConfigBuilder::new();

        config_builder.flags(match (asynchronous, no_rt) {
            (true, true) => ConfigFlagBits::AsyncNoRealtime,
            (true, false) => ConfigFlagBits::AsyncRealtime,
            _ => ConfigFlagBits::Zero,
        });
        config_builder.userdata(Box::into_raw(Box::new(callback)) as *mut std::os::raw::c_void);
        config_builder.handler(Some(handler_cb_wrapper));
        config_builder.speaker(SpeakerID::default()); // No speaker by default

        let config = config_builder.build().unwrap();
        let session = unsafe { afi::aas_create_session(model.ctx, config.into()) };

        if session.is_null() {
            Err("Failed to create ASR session".into())
        } else {
            Ok(Session {
                ctx: session,
                _model: model.into(),
            })
        }
    }

    /// Processes any unprocessed samples and produces a final result.
    ///
    /// # Safety
    ///
    /// The safety of this function depends on the correctness of the `aas_flush` function
    /// from the underlying FFI (Foreign Function Interface) library. Incorrect usage or
    /// invalid parameters may lead to undefined behavior.
    pub fn flush(&self) {
        unsafe { afi::aas_flush(self.ctx) };
    }

    /// Frees the ASR session, saving state to a file if `AprilSpeakerID` was supplied.
    ///
    /// This function calls the `aas_free` function to free the ASR session. It must be called
    /// for all sessions before freeing the model. If an unwinding panic occurs during the free
    /// attempt, the error is caught, and the session is still considered successfully freed.
    /// The error information is printed to the standard error stream.
    ///
    /// # Safety
    ///
    /// The safety of this function depends on the correctness of the `aas_free` function
    /// from the underlying FFI (Foreign Function Interface) library. Incorrect usage or
    /// invalid parameters may lead to undefined behavior.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Session` instance after freeing or an error message.
    pub fn free(&self) {
        unsafe { afi::aas_free(self.ctx) };
    }

    /// Feed PCM16 audio samples to the session.
    ///
    /// This method takes a vector of 8-bit PCM audio samples (`pcm16_samples`) and converts them
    /// to 16-bit signed integers (`i16`). It ensures that every consecutive pair of bytes is
    /// mapped to a single i16 sample using little-endian byte order.
    ///
    /// # Arguments
    ///
    /// * `pcm16_samples` - A vector of 16-bit signed integer PCM audio samples.
    ///
    /// # Safety
    ///
    /// The method internally uses unsafe code to feed the PCM16 audio data to the session.
    /// Ensure that the provided `pcm16_bytes` vector is valid and adheres to the specified format.
    ///
    /// The PCM16 audio data must be single-channel and sampled according to the sample rate
    /// obtained from `aam_get_sample_rate`.
    pub fn feed_pcm16(&self, mut pcm16_bytes: Vec<i16>) {
        unsafe { afi::aas_feed_pcm16(self.ctx, pcm16_bytes.as_mut_ptr(), pcm16_bytes.len()) };
    }

    /// Gets the speedup factor for realtime processing.
    ///
    /// If the `ConfigFlagBits::AsyncRealtime` flag is set, this method returns a floating-point
    /// number describing how much audio is being sped up to keep up with realtime processing.
    /// If the number is below `1.0`, audio is not being sped up. If it is greater than `1.0`,
    /// the audio is being sped up, and the accuracy may be reduced.
    ///
    /// # Safety
    ///
    /// This method is marked as unsafe because it relies on the correctness of the underlying
    /// FFI (Foreign Function Interface) call to `afi::aas_realtime_get_speedup`. Incorrect usage
    /// or invalid parameters may lead to undefined behavior.
    ///
    /// # Returns
    ///
    /// The speedup factor for realtime processing.
    pub fn realtime_get_speedup(&self) -> f32 {
        unsafe { afi::aas_realtime_get_speedup(self.ctx) }
    }
}

/// Implementation of the `Drop` trait for the `Session` struct.
///
/// The `Drop` trait defines a method named `drop` that is called when the value
/// goes out of scope. In this implementation, it is used to free the resources
/// associated with the ASR session, ensuring proper cleanup.
///
/// # Safety
///
/// The `afi::aas_free` function is marked as `unsafe` because it deals with raw
/// pointers and memory management. The implementation assumes that the
/// `aprilasr_sys` crate provides a safe and correct way to free the resources
/// associated with the ASR session. Incorrect usage of this function or invalid
/// pointers may result in undefined behavior.
impl<'a> Drop for Session<'a> {
    fn drop(&mut self) {
        unsafe {
            afi::aas_free(self.ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use md5::compute;
    use std::{ffi::CStr, panic::catch_unwind, ptr::null_mut};

    use super::*;

    #[test]
    fn uses_functional_sys_crate() {
        let ffi_version = afi::APRIL_VERSION as c_int;
        assert_eq!(ffi_version, 1);
        assert_eq!(unsafe { afi::aam_api_init(ffi_version) }, ());
    }

    #[test]
    fn exposes_expected_api_version() {
        let rust_version = APRIL_VERSION;
        assert_ne!(Some(rust_version), None);
        assert_eq!(rust_version, 1);
    }

    #[test]
    fn provides_init_wrapper() -> Result<(), Box<dyn std::error::Error>> {
        let _ = catch_unwind(|| {
            init_april_api(APRIL_VERSION);
        });
        Ok(())
    }

    #[test]
    #[should_panic]
    fn init_rejects_unexpected_versions() {
        init_april_api(0);
    }

    #[test]
    #[should_panic]
    fn init_rejects_unsupported_versions() {
        init_april_api(2);
    }

    #[test]
    fn can_load_model() {
        init_april_api(APRIL_VERSION);

        let path = "april-english-dev-01110_en.april";
        let path_str = CString::new(path).expect("CString::new failed");
        let model = unsafe { afi::aam_create_model(path_str.as_ptr()) };
        assert_ne!(model, null_mut());

        let result = unsafe {
            CStr::from_ptr(afi::aam_get_description(model))
                .to_string_lossy()
                .to_string()
        };
        assert_eq!(result, "Punctuation + Numbers 23a3");

        // Do needless things to demonstrate ways to do useful things.
        let result = unsafe { CStr::from_ptr(afi::aam_get_description(model)).to_str() };
        assert_eq!(result, Ok("Punctuation + Numbers 23a3"));

        // Do needless things to demonstrate ways to do useful things.
        let c_str = unsafe { CStr::from_ptr(afi::aam_get_description(model)) };
        assert_eq!(c_str.to_bytes_with_nul(), b"Punctuation + Numbers 23a3\0");

        // Do needless things to demonstrate ways to do useful things.
        let c_str = unsafe { CStr::from_ptr(afi::aam_get_description(model)) };
        let rust_str = c_str.to_str().expect("Bad encoding");
        let owned = rust_str.to_owned(); // Take ownership of the string
        assert_eq!(c_str.to_bytes_with_nul(), b"Punctuation + Numbers 23a3\0");

        let char_ptr = unsafe { afi::aam_get_name(model) };
        let c_str = unsafe { CStr::from_ptr(char_ptr) };
        let result = c_str.to_string_lossy().to_string();
        assert_eq!(result, "April English Dev-01110");

        let byte_slice = unsafe { CStr::from_ptr(afi::aam_get_language(model)).to_bytes() };
        let result = unsafe { String::from_utf8_unchecked(byte_slice.to_vec()).to_string() };
        assert_eq!(result, "en");

        let result = unsafe { afi::aam_get_sample_rate(model) };
        assert_eq!(result, 16000);

        unsafe { afi::aam_free(model) }

        assert_eq!(owned.as_str(), "Punctuation + Numbers 23a3"); // Assert retained ownership
        assert_ne!(rust_str, "Punctuation + Numbers 23a3"); // Assert lost ownership
    }

    #[test]
    fn cannot_load_fake_model() {
        init_april_api(APRIL_VERSION);

        let path_str = CString::new("invalid.april").expect("CString::new failed");
        let model = unsafe { afi::aam_create_model(path_str.as_ptr()) };
        assert_eq!(model, null_mut());

        // Do needless things to demonstrate ways to do useful things.
        let path_ptr = CString::new("invalid.april").unwrap().into_raw();
        let model = unsafe { afi::aam_create_model(path_ptr) };
        let _ = unsafe { CString::from_raw(path_ptr) };

        unsafe { afi::aam_free(model) }
    }

    #[test]
    fn wraps_model() {
        init_april_api(APRIL_VERSION);

        let model_path = "april-english-dev-01110_en.april";
        let model = Model::new(model_path).unwrap();

        assert_eq!(model.name(), "April English Dev-01110");
        assert_eq!(model.description(), "Punctuation + Numbers 23a3");
        assert_eq!(model.language(), "en");
        assert_eq!(model.sample_rate(), 16000);

        // Drop trait automatically frees model memory.
    }

    #[test]
    fn wraps_result_tokens() {
        let result = Token::new(
            b" BATMAN\0".as_ptr(),
            8.73,
            afi::AprilTokenFlagBits_APRIL_TOKEN_FLAG_WORD_BOUNDARY_BIT,
            1705461067638,
        )
        .unwrap();

        assert_eq!(result.token(), " BATMAN");
        assert_eq!(result.logprob(), 8.73);
        assert_eq!(result.flags(), TokenFlagBits::WordBoundary);
        assert_eq!(result.time_ms(), 1705461067638);

        // Do needless things to demonstrate ways to do useful things.
        let mut logprobs = vec![8.73, 4.62, 9.51];
        logprobs.resize_with(5, Default::default);
        assert_eq!(logprobs, [8.73, 4.62, 9.51, 0., 0.]);

        // Do needless things to demonstrate ways to do useful things.
        let mut flag_bits = vec![
            TokenFlagBits::WordBoundary,
            TokenFlagBits::WordBoundary,
            TokenFlagBits::SentenceEnd,
        ];
        flag_bits.resize_with(5, || TokenFlagBits::WordBoundary);
        assert_eq!(
            flag_bits.last().unwrap().clone(),
            TokenFlagBits::WordBoundary
        );

        let another_result = Token {
            token: String::from(" AND"),
            logprob: logprobs[0],
            flags: flag_bits.last().unwrap().clone(),
            time_ms: result.time_ms() + 320,
        };

        assert_eq!(another_result.token(), " AND");
        assert_eq!(another_result.logprob(), 8.73);
        assert_eq!(another_result.flags(), TokenFlagBits::WordBoundary);
        assert_eq!(another_result.time_ms(), 1705461067958);

        let last_result = Token {
            token: String::from(" ROBIN"),
            logprob: logprobs[2],
            flags: TokenFlagBits::SentenceEnd,
            time_ms: another_result.time_ms() + 230,
        };

        assert_eq!(last_result.token(), " ROBIN");
        assert_eq!(last_result.logprob(), 9.51);
        assert_eq!(last_result.flags(), TokenFlagBits::SentenceEnd);
        assert_eq!(last_result.time_ms(), 1705461068188);
    }

    #[test]
    fn creating_april_token_from_valid_input() {
        let valid_token = CString::new("example_token").expect("CString::new failed");
        let logprob = 0.5;
        let valid_flags = afi::AprilTokenFlagBits_APRIL_TOKEN_FLAG_WORD_BOUNDARY_BIT;
        let time_ms = 100;

        match Token::new(valid_token.as_ptr(), logprob, valid_flags, time_ms) {
            Ok(april_token) => {
                assert_eq!(april_token.token, "example_token");
                assert_eq!(april_token.logprob, 0.5);
                assert_eq!(april_token.flags, TokenFlagBits::WordBoundary);
                assert_eq!(april_token.time_ms, 100);
            }
            Err(_) => {
                panic!("Valid token creation failed unexpectedly");
            }
        }
    }

    #[test]
    fn provides_speaker_interface() {
        // This is a UTF-16LE encoding. The first two bytes 0xAB and 0xCD represent
        // the byte order mark for little-endian UTF-16 (BOM). The remaining bytes
        // are the encoded text as UTF-16LE characters, which are 2-byte code units
        // that consist of a high surrogate followed by a low surrogate.
        let encoded = [
            0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xBA, 0xDC, 0xFE, 0x1A, 0x2B, 0x3C,
            0x4D, 0x5E,
        ];

        #[allow(unused_mut)]
        let mut max_speakers = SpeakerID { data: encoded };
        assert_eq!(max_speakers.data.len(), 16);

        // Do needless things to demonstrate ways to do useful things.
        let cyphertext_speakers = vec![
            "3e6e450acf34e9f3333bfdadb516e533", // echo Jane | md5sum
            "0f36f95c7f1ddfc81ea827400c4a7c2c", // echo John | md5sum
            "2fc1c0beb992cd7096975cfebf9d5c3b", // echo Bob | md5sum
        ];
        let search_value = cyphertext_speakers[1];
        match cyphertext_speakers
            .iter()
            .position(|name| name == &"61409aa1fd47d4a5332de23cbf59a36f")
        {
            Some(index) => println!("Found {} at index {}", search_value, index),
            None => println!("{} not found in list", search_value),
        };

        // Do needless things to demonstrate ways to do useful things.
        let speakers = vec!["Jane", "John", "Bob"];
        let mut speaker_ids = Vec::new();
        for speaker in speakers {
            println!("Encrypting {}", speaker);
            let hash = compute(speaker);
            let id = SpeakerID { data: hash.0 };
            speaker_ids.push(id);
            println!("Added new speaker with hash {:?}", hash);
        }
    }

    fn is_recognition_result(result_type: ResultType) -> bool {
        match result_type {
            ResultType::RecognitionFinal(_) | ResultType::RecognitionPartial(_) => true,
            ResultType::Unknown | ResultType::CantKeepUp | ResultType::Silence => false,
        }
    }

    #[test]
    fn test_recognition_result_types() {
        assert!(is_recognition_result(ResultType::RecognitionPartial(None)));
        assert!(is_recognition_result(ResultType::RecognitionFinal(None)));
        assert!(!is_recognition_result(ResultType::Unknown));
        assert!(!is_recognition_result(ResultType::CantKeepUp));
        assert!(!is_recognition_result(ResultType::Silence));
    }

    #[test]
    fn test_config_builder() {
        // Test default configuration
        let default_config = ConfigBuilder::new().build().unwrap();
        let expected_default = Config {
            speaker: SpeakerID::default(),
            handler: None,
            userdata: ::std::ptr::null_mut(),
            flags: ConfigFlagBits::AsyncNoRealtime,
        };
        assert_eq!(default_config, expected_default);

        // Define a mock handler_callback for testing
        #[allow(unused_variables)]
        fn handler_callback(result_type: ResultType) {
            unimplemented!()
        }

        // Test custom configuration with handler callback
        let custom_config = ConfigBuilder::new()
            .speaker(SpeakerID { data: [42; 16] })
            .handler(Some(handler_cb_wrapper))
            .userdata(handler_callback as *mut std::os::raw::c_void)
            .flags(ConfigFlagBits::Zero)
            .build()
            .unwrap();
        let expected_custom = Config {
            speaker: SpeakerID { data: [42; 16] },
            handler: Some(handler_cb_wrapper),
            userdata: handler_callback as *mut std::os::raw::c_void,
            flags: ConfigFlagBits::Zero,
        };
        assert_eq!(custom_config, expected_custom);

        // Test builder can pass self by mutable reference
        let mut config_builder = ConfigBuilder::new();
        config_builder.speaker(SpeakerID::default());
        config_builder.flags(ConfigFlagBits::AsyncRealtime);
        let mut_config = config_builder.build().unwrap();
        let expected_mut = Config {
            speaker: SpeakerID::default(),
            handler: None,
            userdata: ::std::ptr::null_mut(),
            flags: ConfigFlagBits::AsyncRealtime,
        };
        assert_eq!(mut_config, expected_mut);
    }

    #[test]
    fn wraps_session() {
        init_april_api(APRIL_VERSION);

        let model = Model::new("april-english-dev-01110_en.april").unwrap();

        let session = Session::new(
            &model,
            |result_type| println!("{:?}", result_type),
            true,
            true,
        )
        .unwrap();

        assert!(session.ctx.is_null() == false);

        // Drop traits automatically free session and model memory.
    }

    #[test]
    fn feeds_pcm16_to_session() {
        init_april_api(APRIL_VERSION);

        let model = Model::new("april-english-dev-01110_en.april").unwrap();
        let asynchronous = true;
        let no_rt = true;
        let callback = |result_type| println!("{:?}", result_type);
        let session = Session::new(&model, callback, asynchronous, no_rt).unwrap();

        let empty_vec: Vec<u8> = Vec::new();
        session.feed_pcm16(empty_vec);

        // Drop traits automatically free session and model memory.
    }

    #[test]
    fn test_session_can_be_flushed() {
        init_april_api(APRIL_VERSION);

        let model = Model::new("april-english-dev-01110_en.april").unwrap();
        let asynchronous = true;
        let no_rt = true;
        let callback = |result_type| println!("{:?}", result_type);
        let session = Session::new(&model, callback, asynchronous, no_rt).unwrap();

        // Sessions must be fed before being flushed
        let empty_vec: Vec<u8> = Vec::new();
        session.feed_pcm16(empty_vec);
        session.flush();

        // Drop traits automatically free session and model memory.
    }

    #[test]
    fn test_session_can_check_speedup() {
        init_april_api(APRIL_VERSION);

        let model = Model::new("april-english-dev-01110_en.april").unwrap();
        let asynchronous = true;
        let no_rt = true;
        let callback = |result_type| println!("{:?}", result_type);
        let session = Session::new(&model, callback, asynchronous, no_rt).unwrap();

        // Expects ConfigFlagBits::AsyncRealtime (asynchronyous=true, no_rt=false)
        let speedup = session.realtime_get_speedup();

        assert_eq!(speedup, 1.0);

        // Drop traits automatically free session and model memory.
    }

    #[test]
    fn test_sessions_can_share_model() {
        init_april_api(APRIL_VERSION);

        let model = Model::new("april-english-dev-01110_en.april").unwrap();
        let asynchronous = true;
        let no_rt = true;
        let callback = |result_type| println!("{:?}", result_type);

        let _ = Session::new(&model, callback, asynchronous, no_rt).unwrap();
        let _ = Session::new(&model, callback, asynchronous, no_rt).unwrap();
    }
}
