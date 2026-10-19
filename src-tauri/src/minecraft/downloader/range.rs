use super::error::{ContentRangeError, ContentRangeField, DownloadError};

pub(crate) fn validate_content_range(
    headers: &reqwest::header::HeaderMap,
    expected_start: u64,
    expected_end: u64,
    expected_total: u64,
) -> Result<(), DownloadError> {
    let header = headers
        .get(reqwest::header::CONTENT_RANGE)
        .ok_or(ContentRangeError::MissingHeader)?;
    let value = header.to_str().map_err(ContentRangeError::NonAscii)?;
    let bytes = value
        .strip_prefix("bytes ")
        .ok_or(ContentRangeError::UnsupportedUnit)?;
    let (range, total) = bytes
        .split_once('/')
        .ok_or(ContentRangeError::MissingTotalSeparator)?;
    let (start, end) = range
        .split_once('-')
        .ok_or(ContentRangeError::MissingEndSeparator)?;
    let start = parse_number(start, ContentRangeField::Start)?;
    let end = parse_number(end, ContentRangeField::End)?;
    if start != expected_start || end != expected_end {
        return Err(ContentRangeError::RangeMismatch {
            expected_start,
            expected_end,
            actual_start: start,
            actual_end: end,
        }
        .into());
    }
    if total == "*" {
        return Ok(());
    }
    let total = parse_number(total, ContentRangeField::Total)?;
    if total != expected_total {
        return Err(ContentRangeError::TotalMismatch {
            expected: expected_total,
            actual: total,
        }
        .into());
    }
    Ok(())
}

fn parse_number(value: &str, field: ContentRangeField) -> Result<u64, ContentRangeError> {
    value
        .parse()
        .map_err(|source| ContentRangeError::InvalidNumber { field, source })
}

#[cfg(test)]
mod tests;
