use std::sync::mpsc::Sender;
pub struct State {
    pub length: usize,
    pub already_commanded: bool,
    pub switched_modes: bool,
    pub listening: bool,
    pub infer: bool,
    pub to_ollama: Option<Sender<String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            length: 0,
            already_commanded: true,
            listening: false,
            switched_modes: false,
            infer: false,
            to_ollama: None,
        }
    }
}

impl State {
    pub fn clear(&mut self) {
        self.length = 0;
        self.infer = false;
        self.already_commanded = false;
        self.switched_modes = false;
    }

    pub fn ollama_channel(&mut self, sender: Sender<String>) {
        self.to_ollama.replace(sender);
    }
}
