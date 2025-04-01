use sha2::{Digest, Sha256};

// Separating this function into a new file allows us to choose if we want to implement SHA256
// ourselves as an exercise though not recommended. Well better separation of concerns
#[must_use]
#[inline]
pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    // Convert bytes to hex string manually. result is a 32 byte array
    // I'm surprised that it works but ig the sha2 crate writers implemented LowerHex trait already
    format!("{:x}", result)
}

// Verify that our sha function is correct, could be helpful if I decided to
// implement SHA256 myself
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha_hello() {
        test_helper(
            "hello",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        )
    }

    #[test]
    fn sha_mumei() {
        test_helper(
            "mumei",
            "986facb8d72d3c08b03c2001ec26936bbfc72d844b7965da9badb4a097cc36f3",
        )
    }

    #[test]
    fn sha_azki() {
        test_helper(
            "Azki",
            "e194dca5785eff218c3f29e6667a78f24d4b331b2966b06bc5312d2d04ec84be",
        )
    }

    #[test]
    fn sha_long() {
        test_helper(
            "kNenbnkk873klnnaacbbhynqyqbm",
            "71868123ad34c31cc186ce0220584ab5e09408013fda3a72f886a9b98a150446",
        )
    }

    fn test_helper(data: &str, expected_sha: &str) {
        let data = data.as_bytes();
        let output = calculate_sha256(data);
        assert_eq!(output, expected_sha);
    }
}
