use rs_zephyr_common::{
    log::{LogLevel, ZephyrLog},
    RelayedMessageRequest,
};

use crate::env::EnvClient;

/// Logger object.
pub struct EnvLogger;

impl EnvLogger {
    /// Logs an error to the environment.
    pub fn error(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Error,
            message: message.to_string(),
            data,
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }

    /// Logs a debug event to the environment.
    pub fn debug(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Debug,
            message: message.to_string(),
            data,
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }

    /// Logs a warning to the environment.
    pub fn warning(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Warning,
            message: message.to_string(),
            data,
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }
}
