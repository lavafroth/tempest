use mouse_keyboard_input::key_codes::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use trie_rs::TrieBuilder;

#[derive(Deserialize)]
pub struct RawConfig {
    pub model_path: String,
    pub wake_phrase: String,
    pub rest_phrase: String,
    pub infer_phrase: String,
    pub actions: Vec<RawBinding>,
    pub ollama_model: String,
    pub ollama_endpoint: String,
}

#[derive(Deserialize, Clone)]
pub enum Action {
    Keys(Vec<u16>),
    Command(Vec<String>),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawAction {
    Keys(Vec<String>),
    Command(Vec<String>),
}

impl From<RawAction> for Action {
    fn from(value: RawAction) -> Self {
        match value {
            RawAction::Keys(v) => {
                let v = v.into_iter().map(decode_key).collect();
                Self::Keys(v)
            }
            RawAction::Command(v) => Self::Command(v),
        }
    }
}

#[derive(Deserialize)]
pub struct RawBinding {
    phrase: String,

    #[serde(flatten)]
    action: RawAction,
}

pub struct Config {
    pub model_path: String,
    pub actions: BTreeMap<String, Action>,
    pub word_trie: trie_rs::Trie<u8>,
    pub keys: Vec<String>,
    pub abstract_triggers: trie_rs::Trie<u8>,
    pub modes: BTreeMap<String, Mode>,
    pub ollama_model: String,
    pub ollama_endpoint: String,
}

fn decode_key<S>(key: S) -> u16
where
    S: AsRef<str>,
{
    match key.as_ref().to_uppercase().as_str() {
        "ESC" => KEY_ESC,
        "1" => KEY_1,
        "2" => KEY_2,
        "3" => KEY_3,
        "4" => KEY_4,
        "5" => KEY_5,
        "6" => KEY_6,
        "7" => KEY_7,
        "8" => KEY_8,
        "9" => KEY_9,
        "10" => KEY_10,
        "MINUS" => KEY_MINUS,
        "EQUAL" => KEY_EQUAL,
        "BACKSPACE" => KEY_BACKSPACE,
        "TAB" => KEY_TAB,
        "Q" => KEY_Q,
        "W" => KEY_W,
        "E" => KEY_E,
        "R" => KEY_R,
        "T" => KEY_T,
        "Y" => KEY_Y,
        "U" => KEY_U,
        "I" => KEY_I,
        "O" => KEY_O,
        "P" => KEY_P,
        "LEFTBRACE" => KEY_LEFTBRACE,
        "RIGHTBRACE" => KEY_RIGHTBRACE,
        "ENTER" => KEY_ENTER,
        "LEFTCTRL" => KEY_LEFTCTRL,
        "A" => KEY_A,
        "S" => KEY_S,
        "D" => KEY_D,
        "F" => KEY_F,
        "G" => KEY_G,
        "H" => KEY_H,
        "J" => KEY_J,
        "K" => KEY_K,
        "L" => KEY_L,
        "SEMICOLON" => KEY_SEMICOLON,
        "APOSTROPHE" => KEY_APOSTROPHE,
        "GRAVE" => KEY_GRAVE,
        "LEFTSHIFT" => KEY_LEFTSHIFT,
        "BACKSLASH" => KEY_BACKSLASH,
        "Z" => KEY_Z,
        "X" => KEY_X,
        "C" => KEY_C,
        "V" => KEY_V,
        "B" => KEY_B,
        "N" => KEY_N,
        "M" => KEY_M,
        "COMMA" => KEY_COMMA,
        "DOT" => KEY_DOT,
        "SLASH" => KEY_SLASH,
        "RIGHTSHIFT" => KEY_RIGHTSHIFT,
        "KPASTERISK" => KEY_KPASTERISK,
        "LEFTALT" => KEY_LEFTALT,
        "SPACE" => KEY_SPACE,
        "CAPSLOCK" => KEY_CAPSLOCK,
        "F1" => KEY_F1,
        "F2" => KEY_F2,
        "F3" => KEY_F3,
        "F4" => KEY_F4,
        "F5" => KEY_F5,
        "F6" => KEY_F6,
        "F7" => KEY_F7,
        "F8" => KEY_F8,
        "F9" => KEY_F9,
        "F10" => KEY_F10,
        "NUMLOCK" => KEY_NUMLOCK,
        "SCROLLLOCK" => KEY_SCROLLLOCK,
        "KP7" => KEY_KP7,
        "KP8" => KEY_KP8,
        "KP9" => KEY_KP9,
        "KPMINUS" => KEY_KPMINUS,
        "KP4" => KEY_KP4,
        "KP5" => KEY_KP5,
        "KP6" => KEY_KP6,
        "KPPLUS" => KEY_KPPLUS,
        "KP1" => KEY_KP1,
        "KP2" => KEY_KP2,
        "KP3" => KEY_KP3,
        "KP0" => KEY_KP0,
        "KPDOT" => KEY_KPDOT,

        "ZENKAKUHANKAKU" => KEY_ZENKAKUHANKAKU,
        "102ND" => KEY_102ND,
        "F11" => KEY_F11,
        "F12" => KEY_F12,
        "RO" => KEY_RO,
        "KATAKANA" => KEY_KATAKANA,
        "HIRAGANA" => KEY_HIRAGANA,
        "HENKAN" => KEY_HENKAN,
        "KATAKANAHIRAGANA" => KEY_KATAKANAHIRAGANA,
        "MUHENKAN" => KEY_MUHENKAN,
        "KPJPCOMMA" => KEY_KPJPCOMMA,
        "KPENTER" => KEY_KPENTER,
        "RIGHTCTRL" => KEY_RIGHTCTRL,
        "KPSLASH" => KEY_KPSLASH,
        "SYSRQ" => KEY_SYSRQ,
        "RIGHTALT" => KEY_RIGHTALT,
        "LINEFEED" => KEY_LINEFEED,
        "HOME" => KEY_HOME,
        "UP" => KEY_UP,
        "PAGEUP" => KEY_PAGEUP,
        "LEFT" => KEY_LEFT,
        "RIGHT" => KEY_RIGHT,
        "END" => KEY_END,
        "DOWN" => KEY_DOWN,
        "PAGEDOWN" => KEY_PAGEDOWN,
        "INSERT" => KEY_INSERT,
        "DELETE" => KEY_DELETE,
        "MACRO" => KEY_MACRO,
        "MUTE" => KEY_MUTE,
        "VOLUMEDOWN" => KEY_VOLUMEDOWN,
        "VOLUMEUP" => KEY_VOLUMEUP,
        "POWER" => KEY_POWER,
        "KPEQUAL" => KEY_KPEQUAL,
        "KPPLUSMINUS" => KEY_KPPLUSMINUS,
        "PAUSE" => KEY_PAUSE,
        "SCALE" => KEY_SCALE,

        "KPCOMMA" => KEY_KPCOMMA,
        "HANGEUL" => KEY_HANGEUL,
        "HANGUEL" => KEY_HANGUEL,
        "HANJA" => KEY_HANJA,
        "YEN" => KEY_YEN,
        "LEFTMETA" => KEY_LEFTMETA,
        "RIGHTMETA" => KEY_RIGHTMETA,
        "COMPOSE" => KEY_COMPOSE,

        "STOP" => KEY_STOP,
        "AGAIN" => KEY_AGAIN,
        "PROPS" => KEY_PROPS,
        "UNDO" => KEY_UNDO,
        "FRONT" => KEY_FRONT,
        "COPY" => KEY_COPY,
        "OPEN" => KEY_OPEN,
        "PASTE" => KEY_PASTE,
        "FIND" => KEY_FIND,
        "CUT" => KEY_CUT,
        "HELP" => KEY_HELP,
        "MENU" => KEY_MENU,
        "CALC" => KEY_CALC,
        "SETUP" => KEY_SETUP,
        "SLEEP" => KEY_SLEEP,
        "WAKEUP" => KEY_WAKEUP,
        "FILE" => KEY_FILE,
        "SENDFILE" => KEY_SENDFILE,
        "DELETEFILE" => KEY_DELETEFILE,
        "XFER" => KEY_XFER,
        "PROG1" => KEY_PROG1,
        "PROG2" => KEY_PROG2,
        "WWW" => KEY_WWW,
        "MSDOS" => KEY_MSDOS,
        "COFFEE" => KEY_COFFEE,
        "SCREENLOCK" => KEY_SCREENLOCK,
        "ROTATE_DISPLAY" => KEY_ROTATE_DISPLAY,
        "DIRECTION" => KEY_DIRECTION,
        "CYCLEWINDOWS" => KEY_CYCLEWINDOWS,
        "MAIL" => KEY_MAIL,
        "BOOKMARKS" => KEY_BOOKMARKS,
        "COMPUTER" => KEY_COMPUTER,
        "BACK" => KEY_BACK,
        "FORWARD" => KEY_FORWARD,
        "CLOSECD" => KEY_CLOSECD,
        "EJECTCD" => KEY_EJECTCD,
        "EJECTCLOSECD" => KEY_EJECTCLOSECD,
        "NEXTSONG" => KEY_NEXTSONG,
        "PLAYPAUSE" => KEY_PLAYPAUSE,
        "PREVIOUSSONG" => KEY_PREVIOUSSONG,
        "STOPCD" => KEY_STOPCD,
        "RECORD" => KEY_RECORD,
        "REWIND" => KEY_REWIND,
        "PHONE" => KEY_PHONE,
        "ISO" => KEY_ISO,
        "CONFIG" => KEY_CONFIG,
        "HOMEPAGE" => KEY_HOMEPAGE,
        "REFRESH" => KEY_REFRESH,
        "EXIT" => KEY_EXIT,
        "MOVE" => KEY_MOVE,
        "EDIT" => KEY_EDIT,
        "SCROLLUP" => KEY_SCROLLUP,
        "SCROLLDOWN" => KEY_SCROLLDOWN,
        "KPLEFTPAREN" => KEY_KPLEFTPAREN,
        "KPRIGHTPAREN" => KEY_KPRIGHTPAREN,
        "NEW" => KEY_NEW,
        "REDO" => KEY_REDO,

        "F13" => KEY_F13,
        "F14" => KEY_F14,
        "F15" => KEY_F15,
        "F16" => KEY_F16,
        "F17" => KEY_F17,
        "F18" => KEY_F18,
        "F19" => KEY_F19,
        "F20" => KEY_F20,
        "F21" => KEY_F21,
        "F22" => KEY_F22,
        "F23" => KEY_F23,
        "F24" => KEY_F24,

        "PLAYCD" => KEY_PLAYCD,
        "PAUSECD" => KEY_PAUSECD,
        "PROG3" => KEY_PROG3,
        "PROG4" => KEY_PROG4,
        "DASHBOARD" => KEY_DASHBOARD,
        "SUSPEND" => KEY_SUSPEND,
        "CLOSE" => KEY_CLOSE,
        "PLAY" => KEY_PLAY,
        "FASTFORWARD" => KEY_FASTFORWARD,
        "BASSBOOST" => KEY_BASSBOOST,
        "PRINT" => KEY_PRINT,
        "HP" => KEY_HP,
        "CAMERA" => KEY_CAMERA,
        "SOUND" => KEY_SOUND,
        "QUESTION" => KEY_QUESTION,
        "EMAIL" => KEY_EMAIL,
        "CHAT" => KEY_CHAT,
        "SEARCH" => KEY_SEARCH,
        "CONNECT" => KEY_CONNECT,
        "FINANCE" => KEY_FINANCE,
        "SPORT" => KEY_SPORT,
        "SHOP" => KEY_SHOP,
        "ALTERASE" => KEY_ALTERASE,
        "CANCEL" => KEY_CANCEL,
        "BRIGHTNESSDOWN" => KEY_BRIGHTNESSDOWN,
        "BRIGHTNESSUP" => KEY_BRIGHTNESSUP,
        "MEDIA" => KEY_MEDIA,

        "SWITCHVIDEOMODE" => KEY_SWITCHVIDEOMODE,
        "KBDILLUMTOGGLE" => KEY_KBDILLUMTOGGLE,
        "KBDILLUMDOWN" => KEY_KBDILLUMDOWN,
        "KBDILLUMUP" => KEY_KBDILLUMUP,

        "SEND" => KEY_SEND,
        "REPLY" => KEY_REPLY,
        "FORWARDMAIL" => KEY_FORWARDMAIL,
        "SAVE" => KEY_SAVE,
        "DOCUMENTS" => KEY_DOCUMENTS,

        "BATTERY" => KEY_BATTERY,

        "BLUETOOTH" => KEY_BLUETOOTH,
        "WLAN" => KEY_WLAN,
        "UWB" => KEY_UWB,

        "UNKNOWN" => KEY_UNKNOWN,

        "VIDEO_NEXT" => KEY_VIDEO_NEXT,
        "VIDEO_PREV" => KEY_VIDEO_PREV,
        "BRIGHTNESS_CYCLE" => KEY_BRIGHTNESS_CYCLE,
        "BRIGHTNESS_AUTO" => KEY_BRIGHTNESS_AUTO,
        "BRIGHTNESS_ZERO" => KEY_BRIGHTNESS_ZERO,
        "DISPLAY_OFF" => KEY_DISPLAY_OFF,

        "WWAN" => KEY_WWAN,
        "WIMAX" => KEY_WIMAX,
        "RFKILL" => KEY_RFKILL,

        "MICMUTE" => KEY_MICMUTE,

        "SUPER" => KEY_LEFTMETA,
        _ => KEY_ENTER,
    }
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
            .map(|b| (b.phrase.to_lowercase(), b.action.into()))
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
            model_path: value.model_path,
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
