use std::collections::LinkedList;

#[derive(Debug)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub prevhash: String,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub blocks: LinkedList<Block>, // TODO: replace to own struct
}
