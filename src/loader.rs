// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::time::Duration;

use crate::location::Location;

pub const MAX_DOCUMENT_SIZE: u64 = 10 * 1024 * 1024;
pub const MAX_IMAGE_SIZE: u64 = 20 * 1024 * 1024;
pub const MAX_FONT_SIZE: u64 = 10 * 1024 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const USER_AGENT: &str = "Foundation/0.1";

pub fn fetch(location: &Location, limit: u64) -> Result<Vec<u8>, String> {
    match location {
        Location::Url(url) => fetch_url(url, limit),
        Location::File(path) => {
            let metadata = fs::metadata(path)
                .map_err(|error| format!("cannot read {}: {}", path.display(), error))?;
            if metadata.len() > limit {
                return Err(format!("{} exceeds {} bytes", path.display(), limit));
            }
            fs::read(path).map_err(|error| format!("cannot read {}: {}", path.display(), error))
        }
    }
}

fn fetch_url(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .new_agent();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("cannot fetch {}: {}", url, error))?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| format!("cannot read body of {}: {}", url, error))
}
