//! AES-128-CBC, the one encryption HLS defines for whole segments
//! (`#EXT-X-KEY:METHOD=AES-128`). It is not DRM: the key is served to any
//! player as a plain 16-byte file. SAMPLE-AES and key systems such as
//! Widevine or FairPlay are refused by the parser and never reach this.

use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};

use crate::DownloadError;

type Decryptor = cbc::Decryptor<Aes128>;

/// Decrypt one segment in place and drop its PKCS#7 padding.
pub(crate) fn decrypt(
    key: &[u8; 16],
    iv: &[u8; 16],
    data: &mut Vec<u8>,
) -> Result<(), DownloadError> {
    let len = Decryptor::new(key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(data)
        .map_err(|_| {
            DownloadError::InvalidManifest("a segment does not decrypt with its key".into())
        })?
        .len();
    data.truncate(len);
    Ok(())
}

/// A key as served: exactly 16 bytes.
pub(crate) fn key(bytes: &[u8]) -> Result<[u8; 16], DownloadError> {
    bytes.try_into().map_err(|_| {
        DownloadError::InvalidManifest(format!(
            "an AES-128 key must be 16 bytes, the server sent {}",
            bytes.len()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbc::cipher::BlockEncryptMut;
    use cbc::cipher::block_padding::NoPadding;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn matches_the_nist_cbc_vector() {
        // NIST SP 800-38A, F.2.2 CBC-AES128.Decrypt, first two blocks.
        let key: [u8; 16] = hex("2b7e151628aed2a6abf7158809cf4f3c").try_into().unwrap();
        let iv: [u8; 16] = hex("000102030405060708090a0b0c0d0e0f").try_into().unwrap();
        let mut data = hex("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2");
        let plain = Decryptor::new(&key.into(), &iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut data)
            .unwrap()
            .to_vec();
        assert_eq!(
            plain,
            hex("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51")
        );
    }

    #[test]
    fn round_trips_with_padding() {
        let key = [7u8; 16];
        let iv = [9u8; 16];
        let plain: Vec<u8> = (0..1000u32).map(|i| i as u8).collect();
        let mut buf = plain.clone();
        buf.resize(plain.len() + 16, 0);
        let len = cbc::Encryptor::<Aes128>::new(&key.into(), &iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
            .unwrap()
            .len();
        buf.truncate(len);
        decrypt(&key, &iv, &mut buf).unwrap();
        assert_eq!(buf, plain);
    }

    #[test]
    fn refuses_bad_input() {
        assert!(decrypt(&[0; 16], &[0; 16], &mut vec![1, 2, 3]).is_err());
        assert!(key(&[0; 15]).is_err());
        assert_eq!(key(&[1; 16]).unwrap(), [1; 16]);
    }
}
