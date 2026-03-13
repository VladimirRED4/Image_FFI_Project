use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Input file not found: {0}")]
    InputFileNotFound(PathBuf),

    #[error("Parameters file not found: {0}")]
    ParamsFileNotFound(PathBuf),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Failed to load image: {0}")]
    ImageLoadError(String),

    #[error("Failed to save image: {0}")]
    ImageSaveError(String),

    #[error("Plugin loading error: {0}")]
    PluginLoadError(String),

    #[error("Failed to read parameters file: {0}")]
    ParamsReadError(String),
}