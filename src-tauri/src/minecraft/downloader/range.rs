use super::error::DownloadError;

pub(crate) fn validate_content_range(
    headers: &reqwest::header::HeaderMap,
    expected_start: u64,
    expected_end: u64,
    expected_total: u64,
) -> Result<(), DownloadError> {
    let header_val = headers.get(reqwest::header::CONTENT_RANGE).ok_or_else(|| {
        DownloadError::ContentRange(
            "Missing Content-Range header in 206 Partial Content response".to_string(),
        )
    })?;

    let s = header_val
        .to_str()
        .map_err(|_| DownloadError::ContentRange("Non-ASCII Content-Range header".to_string()))?;

    let bytes_part = s.strip_prefix("bytes ").ok_or_else(|| {
        DownloadError::ContentRange(format!("Invalid Content-Range unit in: {s}"))
    })?;

    let (range_part, total_part) = bytes_part.split_once('/').ok_or_else(|| {
        DownloadError::ContentRange(format!("Malformed Content-Range (missing '/'): {s}"))
    })?;

    let (start_str, end_str) = range_part.split_once('-').ok_or_else(|| {
        DownloadError::ContentRange(format!("Malformed range in Content-Range: {s}"))
    })?;

    let start: u64 = start_str.parse().map_err(|_| {
        DownloadError::ContentRange(format!("Invalid start offset in Content-Range: {s}"))
    })?;

    let end: u64 = end_str.parse().map_err(|_| {
        DownloadError::ContentRange(format!("Invalid end offset in Content-Range: {s}"))
    })?;

    if start != expected_start || end != expected_end {
        return Err(DownloadError::ContentRange(format!(
            "Content-Range mismatch: requested {expected_start}-{expected_end}, got {start}-{end}"
        )));
    }

    if total_part != "*" {
        let total: u64 = total_part.parse().map_err(|_| {
            DownloadError::ContentRange(format!("Invalid total length in Content-Range: {s}"))
        })?;
        if total != expected_total {
            return Err(DownloadError::ContentRange(format!(
                "Content-Range total length mismatch: expected {expected_total}, got {total}"
            )));
        }
    }

    Ok(())
}
