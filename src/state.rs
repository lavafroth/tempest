use std::sync::mpsc::Sender;
#[derive(Default)]
pub struct State {
    pub position: usize,
    pub length: usize,
    pub already_commanded: bool,
    pub listening: bool,
    pub infer: bool,
    pub to_ollama: Option<Sender<String>>,
}

impl State {
    pub fn clear(&mut self) {
        self.length = 0;
        self.position = 0;
        self.infer = false;
        self.already_commanded = false;
    }

    pub fn ollama_channel(&mut self, sender: Sender<String>) {
        self.to_ollama.replace(sender);
    }
}
