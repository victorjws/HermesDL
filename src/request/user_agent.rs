use super::constant;
use anyhow::{anyhow, Error};
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum UserAgent {
    Firefox,
    Chrome,
}

impl UserAgent {
    fn to_header_value(&self) -> &'static str {
        match self {
            UserAgent::Firefox => constant::FIREFOX_USER_AGENT,
            UserAgent::Chrome => constant::CHROME_USER_AGENT,
        }
    }
}

impl Into<HeaderValue> for UserAgent {
    fn into(self) -> HeaderValue {
        HeaderValue::from_static(self.to_header_value())
    }
}

impl FromStr for UserAgent {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            constant::FIREFOX => Ok(UserAgent::Firefox),
            constant::CHROME => Ok(UserAgent::Chrome),
            _ => Err(anyhow!(constant::USER_AGENT_PARSE_ERROR)),
        }
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            UserAgent::Firefox => constant::FIREFOX,
            UserAgent::Chrome => constant::CHROME,
        };
        write!(f, "{}", s)
    }
}
