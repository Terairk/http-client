use std::env;

use crate::client::download_full_data;
use crate::sha::calculate_sha256;
use error::DownloadError;

mod client;
mod error;
mod sha;

fn main() -> Result<(), DownloadError> {
    let args: Vec<String> = env::args().collect();
    // Expect 4 arguments: command, address, hash, size
    // Hash and size are printed by the server so might as well use it
    // Technically speaking, we don't need the hash as we could verify manually but makes it easier
    // to check our work
    // Furthermore, we don't need the size to be passed at the CLI, because
    // judging by the Python HTTP Server: We could just get the total length by not passing in a
    // range initially. But this reduces my burden slightly.
    // Unfortunately the server doesn't follow the HTTP Specification where it should actually send
    // a Content-Range header if a range is being sent to it. ie Content-Range:
    // <start>-<end>/<total>
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <server_address> <total_size_bytes> <expected_sha256_hash>",
            args[0]
        );
        eprintln!("Example: {} 127.0.0.1:8080 450 986f52d9...", args[0]);
        return Err(DownloadError::Args("Invalid number of arguments".into()));
    }

    let server_address = &args[1];
    let total_size: u64 = args[2].parse().map_err(|_| {
        DownloadError::Args(format!(
            "Invalid total size provided: {}. Must be a non-negative integer",
            args[2]
        ))
    })?;
    let expected_hash = args[3].to_lowercase();

    println!("Target Server: {server_address}");
    println!("Expected SHA-256 Hash: {expected_hash}");
    println!("Expected Total Size: {total_size} bytes");

    // Download data using the provided total_size. Largest function by far
    let downloaded_data = download_full_data(server_address, total_size)?;

    // Verify downloaded size just in case (sanity check, perhaps remove this later)
    if downloaded_data.len() as u64 != total_size {
        return Err(DownloadError::Logic(format!(
            "Final downloaded data size ({}) does not match expected size ({})",
            downloaded_data.len(),
            total_size,
        )));
    }

    // Calculate hash
    println!("Calculating SHA-256 hash of downloaded data...");
    let actual_hash = calculate_sha256(&downloaded_data);
    println!("Actual SHA-256:   {actual_hash}");

    // Compare hashes together, hope they match
    if actual_hash != expected_hash {
        return Err(DownloadError::HashMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }

    println!("\nSuccess! Downloaded data matches the expected hash.");
    Ok(())
}
