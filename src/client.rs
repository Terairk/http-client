use crate::error::DownloadError;

// Change i32 to something else later
#[must_use]
#[inline]
pub fn download_full_data(server_address: &str, total_size: u64) -> Result<Vec<u8>, DownloadError> {
    todo!()
}
