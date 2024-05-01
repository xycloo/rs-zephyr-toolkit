use rs_zephyr_common::{log::{LogLevel, ZephyrLog}, RelayedMessageRequest};

use crate::env::EnvClient;

pub struct EnvLogger;

impl EnvLogger {
    pub fn error(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Error,
            message: message.to_string(),
            data
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }

    pub fn debug(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Debug,
            message: message.to_string(),
            data
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }

    pub fn warning(&self, message: impl ToString, data: Option<Vec<u8>>) {
        let log = ZephyrLog {
            level: LogLevel::Warning,
            message: message.to_string(),
            data
        };

        EnvClient::message_relay(RelayedMessageRequest::Log(log));
    }
}