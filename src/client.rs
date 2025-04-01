use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::TcpStream,
    str, thread,
    time::Duration,
};

use crate::error::DownloadError;
const CHUNK_SIZE: u64 = 32 * 1024; // 32 KiB chunk size to not truncate
const MAX_RETRIES: u32 = 10; // Max retries per chunk
const RETRY_DELAY: Duration = Duration::from_millis(500);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const SERVER_ADDR: &str = "127.0.0.1:8080";

struct HttpResponse {
    status_code: u16,
    // diagnostic headers, may be helpful down the line
    // headers: Vec<(String, String)>,
    body: Vec<u8>,
}

// For now keep this function signature if we ever need to give Errors
#[inline]
pub fn download_full_data(total_size: u64) -> Result<Vec<u8>, DownloadError> {
    if total_size == 0 {
        return Ok(Vec::new());
    }

    println!("Attempting to download {total_size} bytes...");
    let mut full_data = vec![0u8; total_size as usize];
    let mut current_pos: u64 = 0;

    println!("Starting download in chunks ofup to {CHUNK_SIZE} bytes...");
    while current_pos < total_size {
        let chunk_start = current_pos;
        let mut chunk_end = current_pos.saturating_add(CHUNK_SIZE).saturating_sub(1);
        if chunk_end >= total_size {
            chunk_end = total_size.saturating_sub(1);
        }

        // A bit of defensive programming here. Catch bugs early.
        // Could instead make a Logic Variant for DownloadError so clients could give better
        // diagonistics if things fail but ideally those never happen.
        debug_assert!(chunk_start <= chunk_end, "Chunk start is after end");
        let chunk_data = download_chunk(SERVER_ADDR, chunk_start, chunk_end)?;
        let expected_len = (chunk_end - chunk_start + 1) as usize;

        // This implementation here would need to change if the server was a block_box
        // that changed its threshold for truncating every time versus being a constant
        debug_assert!(
            chunk_data.len() == expected_len,
            "Downloaded chunk doesn't match the expected size"
        );

        // Copy the downloaded chunk into the correct position in the main buffer
        let start_idx = chunk_start as usize;

        // Defensive programming that we're not writing beyond buffer bounds
        debug_assert!(
            start_idx + chunk_data.len() <= full_data.len(),
            "Attempting to write chunk beyond buffer bounds. end_idx={}, buffer_len={}",
            start_idx + chunk_data.len(),
            full_data.len()
        );

        // Now we know the copy will be valid
        full_data[start_idx..start_idx + chunk_data.len()].copy_from_slice(&chunk_data);

        current_pos += chunk_data.len() as u64;

        // Progress indicator
        let percentage = (current_pos as f64 / total_size as f64) * 100.0;
        print!(
            "\rDownloaded: {:.2}% ({}/{}) bytes",
            percentage, current_pos, total_size
        );
        io::stdout().flush()?; // Ensure progress is displayed immediately
    }

    println!("\nDownload complete.");
    Ok(full_data)
}

// This does some retrying in case downloading fails
fn download_chunk(address: &str, start: u64, end: u64) -> Result<Vec<u8>, DownloadError> {
    let expected_len = (end.saturating_sub(start) + 1) as usize;
    if expected_len == 0 {
        // Shouldn't happen but handle defensively
        return Ok(Vec::new());
    }

    // Debug printing
    // println!("Requesting chunk: bytes={}-{} (expecting {} bytes)", start, end, expected_len);

    for attempt in 1..=MAX_RETRIES {
        // The +1 is because the buggy python server doesn't
        // actually respect the HTTP Range header
        // correctly I think, I might be wrong though
        let request_end = end.saturating_add(1);
        match send_request(address, Some((start, request_end))) {
            Ok(response) => {
                // Debug print
                // println!("Chunk Response Status: {}, Body Length: {}", response.status_code, response.body.len());

                // We expect 206 Partial Content, though server may sometimes send 200 OK if it
                // sends the entire file even for a range request. We only care if the body length
                // matches what we asked for

                if response.status_code == 206 || response.status_code == 200 {
                    if response.body.len() == expected_len {
                        // Debug print
                        // println!("Successfully received chunk{}-{}", start, end);

                        return Ok(response.body);
                    } else {
                        // Received 200/206 but server truncated the body so it doesn't match the
                        // expeced length

                        eprintln!(
                            "Warning: Received truncated chunk ({} bytes) for range {}-{} (expected {}). Retrying (attempt {}/{})",
                            response.body.len(), start, end, expected_len, attempt, MAX_RETRIES
                        );
                        // Fall through to retry delay
                    }
                } else {
                    // Unexpected status code that we don't know how to handle
                    eprintln!(
                        "Error: Received unexpected status {} for range {}-{}. Retrying (attempt {}/{})",
                        response.status_code, start, end, attempt, MAX_RETRIES
                    );
                    // Log body if it might contain error details
                    if !response.body.is_empty() {
                        if let Ok(body_str) = str::from_utf8(&response.body) {
                            eprintln!(
                                "Response body hint: {}",
                                body_str.chars().take(100).collect::<String>()
                            );
                        }
                    }
                    // Fall through to retry delay
                }
            }
            Err(e) => {
                // Handle the network or parsing error
                eprintln!(
                    "Error downloading chunk {}-{}: {}. Retrying (attempt {}/{})",
                    start, end, e, attempt, MAX_RETRIES
                );
                // Fall through to retry delay
            }
        }

        // Wait for a bit before retrying for this chunk
        thread::sleep(RETRY_DELAY);
    }

    // If loop finishes all times then all the retries failed
    Err(DownloadError::Network(format!(
        "Failed to download chunk {start}-{end} after {MAX_RETRIES} retries"
    )))
}

// None means no Range provided, (start_inclusive, end_exclusive)
fn send_request(address: &str, range: Option<(u64, u64)>) -> Result<HttpResponse, DownloadError> {
    // Perhaps we should construct this server_addr earlier so we don't parse as often
    // Also so that we return an error earlier
    let server_addr = address
        .parse()
        .map_err(|_| DownloadError::Args(format!("Invalid server address format: {address}")))?;
    let mut stream = TcpStream::connect_timeout(&server_addr, CONNECT_TIMEOUT)
        .map_err(|e| DownloadError::Network(format!("Failed to connect to {address}: {e}")))?;

    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))?;

    let host = address.split(':').next().unwrap_or(address);

    // TODO: make my own custom builder for this as these all have repeat \r\n
    let mut request_str = format!(
        "GET / HTTP/1.1\r\n\
         Host: {host}\r\n\
         Connection: close\r\n"
    );

    // Format the Range header value
    if let Some((start, end)) = range {
        // Ensure end is greater than start for a valid range request to the server
        if end > start {
            request_str.push_str(&format!("Range: bytes={}-{}\r\n", start, end));
        } else {
            // Not sure if this works but maybe
            request_str.push_str(&format!("Range: bytes={}-{}\r\n", start, start));
        }
    } // No Range header if range is None

    request_str.push_str("\r\n");

    // Debug print the request being sent
    // println!("--- Sending Request ---\n{}-----------------------", request_str);

    stream.write_all(request_str.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    parse_http_response(&mut reader)
}

fn parse_http_response<R: Read>(reader: &mut BufReader<R>) -> Result<HttpResponse, DownloadError> {
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;

    // Status line is "HTTP/1.1 200 OK", so split into 3 parts
    let parts: Vec<&str> = status_line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(DownloadError::Parse(format!(
            "Malformed status line: {status_line}"
        )));
    }

    let status_code: u16 = parts[1]
        .parse()
        .map_err(|_| DownloadError::Parse(format!("Invalid status code: {}", parts[1])))?;

    let mut headers = Vec::new();
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line)?;
        let trimmed_line = header_line.trim();
        if trimmed_line.is_empty() {
            break; // End of headers
        }
        if let Some((name, value)) = trimmed_line.split_once(": ") {
            // store as lowercase for standardized lookup
            headers.push((name.to_lowercase(), value.trim().to_string()));
        } else {
            // Invalid header format
            return Err(DownloadError::Parse(format!(
                "Malformed header line: {header_line}"
            )));
        }
    }

    // May be useful in the future
    // let response_headers = headers.clone();

    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok());

    let mut body = Vec::new();

    if let Some(len) = content_length {
        body.resize(len, 0);
        // Use read_exact to ensure all bytes are read or an error occurs
        match reader.read_exact(&mut body) {
            Ok(_) => {} // Successfully read the body
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // This is the specific case where the server closed the connection
                // before sending `Content-Length` bytes. We still need the partial body.
                // `read_exact` would have filled part of the buffer. We need to know how much.
                // Unfortunately, BufReader doesn't easily expose this after read_exact fails.
                // A lower-level read loop would be needed to capture partial data on UnexpectedEof.
                // For simplicity here, we'll return an error, but acknowledge this limitation.
                // A more robust solution might involve reading byte-by-byte or in smaller chunks
                // within parse_http_response if Content-Length is present.
                // However, our retry logic in `download_chunk` handles this specific server's
                // truncation issue *before* we get here, by checking body.len() vs expected_len.
                // TODO: handle variable amount of bytes sent which requires modifying
                // multiple functions. Basically slightly modified approach is required
                return Err(DownloadError::Network(format!(
                        "Server connection closed prematurely after promising {len} bytes (status {status_code})"
                    )));
            }
            Err(e) => return Err(DownloadError::Io(e)), // Other IO error
        }
    } else {
        // Handle cases where Content-Length might be missing (e.g., HEAD request, 204 No Content, 304 Not Modified)
        // The buggy server *should* always send Content-Length for 200/206, so absence is likely an error.
        if !(status_code == 204 || status_code == 304 || (100..200).contains(&status_code)) {
            return Err(DownloadError::Http(format!(
                "Missing Content-Length header for status {}",
                status_code
            )));
        }
    }

    Ok(HttpResponse {
        status_code,
        // headers: response_headers,
        body,
    })
}

// impl HttpResponse {
//     // Get's the value of a header
//     fn get_header(&self, name: &str) -> Option<&str> {
//         self.headers
//             .iter()
//             .find(|(h_name, _)| h_name.eq_ignore_ascii_case(name))
//             .map(|(_, h_value)| h_value.as_str())
//     }
// }
