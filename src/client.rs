use std::{
    io::{self, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
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

// For now keep this function signature if we ever need to give Errors
#[inline]
pub fn download_full_data(total_size: u64) -> Result<Vec<u8>, DownloadError> {
    if total_size == 0 {
        return Ok(Vec::new());
    }

    println!("Attempting to download {total_size} bytes...");
    // Create buffer of the correct size for efficiency
    let mut full_data = vec![0u8; total_size as usize];
    let mut current_pos: u64 = 0;

    println!("Starting download in chunks of up to {CHUNK_SIZE} bytes...");

    // Create a single TCP connection that we'll try to reuse
    while current_pos < total_size {
        let chunk_start = current_pos;
        let mut chunk_end = current_pos.saturating_add(CHUNK_SIZE).saturating_sub(1);
        if chunk_end >= total_size {
            chunk_end = total_size.saturating_sub(1);
        }
        let chunk_end = chunk_end;

        // A bit of defensive programming here. Catch bugs early.
        // Could instead make a Logic Variant for DownloadError so clients could give better
        // diagonistics if things fail but ideally those never happen.
        debug_assert!(chunk_start <= chunk_end, "Chunk start is after end");
        let chunk_data = download_chunk(chunk_start, chunk_end)?;
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
fn download_chunk(start: u64, end: u64) -> Result<Vec<u8>, DownloadError> {
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
        match send_request(start, request_end) {
            Ok(body) => {
                if body.len() == expected_len {
                    // Debug print
                    // println!("Successfully received chunk{}-{}", start, end);
                    return Ok(body);
                } else {
                    // Received 200/206 but server truncated the body so it doesn't match the
                    // expeced length

                    eprintln!(
                            "Warning: Received truncated chunk ({} bytes) for range {}-{} (expected {}). Retrying (attempt {}/{})",
                            body.len(), start, end, expected_len, attempt, MAX_RETRIES
                        );
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

fn send_request(start: u64, end: u64) -> Result<Vec<u8>, DownloadError> {
    let server_addr: SocketAddr = SERVER_ADDR.parse().expect("SERVER_ADDR is valid");
    let mut stream = TcpStream::connect_timeout(&server_addr, CONNECT_TIMEOUT)?;

    // Format and send HTTP request
    let request = format!(
        "GET / HTTP/1.1\r\n\
         Host: {SERVER_ADDR}\r\n\
         Range: bytes={start}-{end}\r\n\
         Connection: close\r\n\
         \r\n"
    );
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))?;

    stream.write_all(request.as_bytes())?;

    let mut reader = BufReader::new(stream);
    let mut response = Vec::new();
    reader.read_to_end(&mut response)?;

    const DELIMITER: &[u8] = b"\r\n\r\n";

    // Find the end of headers (double CRLF), body is afterwards from it
    match response
        .windows(DELIMITER.len())
        .position(|w| w == b"\r\n\r\n")
    {
        Some(pos) => {
            let body = pos + DELIMITER.len();
            Ok(response[body..].to_vec())
        }
        None => Err(DownloadError::Parse(
            "Chunk has no end of headers therefore no body".to_owned(),
        )),
    }
}
