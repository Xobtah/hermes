use serde::{Deserialize, Serialize};

pub mod api;
pub mod crypto;

#[derive(Debug, Serialize, Deserialize)]
pub struct Mission {
    pub id: i32,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Task {
    Update(Vec<u8>),
    Execute(String),
    Stop,
}
