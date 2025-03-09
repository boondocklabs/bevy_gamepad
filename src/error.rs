#[derive(thiserror::Error, Debug)]
pub enum GamepadError {
    #[error("platform: {0}")]
    Platform(String),
}
