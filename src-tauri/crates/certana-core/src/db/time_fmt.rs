// SPDX-License-Identifier: GPL-3.0-or-later
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::{CertanaError, Result};

pub fn format_ts(t: OffsetDateTime) -> String {
    t.to_offset(time::UtcOffset::UTC)
        .replace_nanosecond(0)
        .expect("valid")
        .format(&Rfc3339)
        .expect("rfc3339 formatting cannot fail")
}

pub fn parse_ts(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| CertanaError::Invalid(format!("bad timestamp {s}: {e}")))
}
