/// Decompression utilities for LZMA-compressed tar archives containing CDEC data
use crate::error::{CdecError, Result};
use lzma_rs::xz_decompress;
use std::io::{BufReader, Read};
use tar::Archive;

/// Embedded cumulative statewide observations (v1)
pub static CUMULATIVE_OBJECT: &[u8] = include_bytes!("../../fixtures/cumulative.tar.lzma");

/// Embedded cumulative statewide observations (v2 - updated data)
pub static CUMULATIVE_OBJECT_V2: &[u8] = include_bytes!("../../fixtures/cumulative_v2.tar.lzma");

/// Embedded per-reservoir observations
pub static OBSERVATIONS_OBJECT: &[u8] = include_bytes!("../../fixtures/reservoirs.tar.lzma");

/// Decompresses an LZMA-compressed tar archive and extracts the first file as a CSV string
///
/// This function performs three steps:
/// 1. Decompresses the LZMA (xz) archive
/// 2. Extracts the tar archive
/// 3. Reads the first file in the archive (expected to be CSV data)
///
/// # Arguments
///
/// * `input` - Raw bytes of the LZMA-compressed tar archive
///
/// # Returns
///
/// The contents of the first file in the archive as a byte vector
///
/// # Errors
///
/// Returns `CdecError::Decompression` if decompression fails
/// Returns `CdecError::TarExtraction` if tar extraction fails
///
/// # Example
///
/// ```no_run
/// use cdec::compression::{decompress_tar_file_to_csv_string, CUMULATIVE_OBJECT_V2};
///
/// let csv_data = decompress_tar_file_to_csv_string(CUMULATIVE_OBJECT_V2)?;
/// # Ok::<(), cdec::CdecError>(())
/// ```
pub fn decompress_tar_file_to_csv_string(input: &[u8]) -> Result<Vec<u8>> {
    // Step 1: Decompress LZMA
    let mut tar_object_buffer = BufReader::new(input);
    let mut decompress_output: Vec<u8> = Vec::new();
    xz_decompress(&mut tar_object_buffer, &mut decompress_output)
        .map_err(|e| CdecError::Decompression(format!("LZMA decompression failed: {}", e)))?;

    // Step 2: Extract tar archive
    let mut tar_file = Archive::new(decompress_output.as_slice());
    let mut entries = tar_file.entries()?;

    // Step 3: Read first file
    if let Some(entry_result) = entries.next() {
        let mut csv_file = entry_result?;
        let mut buf: Vec<u8> = Vec::new();
        csv_file.read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        Err(CdecError::Decompression(
            "Tar archive is empty".to_string(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::decompress_tar_file_to_csv_string;
    use hex_literal::hex;
    use sha3::{Digest, Sha3_384};

    pub static TAR_TEST_OBJECT: &[u8] = include_bytes!("../../test-fixtures/output.tar.lzma");

    #[test]
    fn test_decompress_tar_file_to_csv_string() {
        let output = decompress_tar_file_to_csv_string(TAR_TEST_OBJECT)
            .expect("Failed to decompress test object");

        let mut hasher = Sha3_384::new();
        let bytes = output.as_slice();
        hasher.update(bytes);
        let result = hasher.finalize();

        assert_eq!(
            result[..],
            hex!("35f323d919c0c9ef3bd00f2421c28195506eb67cc971e7a9e3529742337ffdff3636ce839035fa273d90301245fff39d")
        );
    }
}
