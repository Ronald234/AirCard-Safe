use anyhow::{Result, bail};
use std::ffi::c_void;
use std::ptr;
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: *mut c_void, data: *mut u8, len: u32, flags: u32) -> i32;
    fn BCryptOpenAlgorithmProvider(
        out: *mut *mut c_void,
        id: *const u16,
        implementation: *const u16,
        flags: u32,
    ) -> i32;
    fn BCryptCloseAlgorithmProvider(handle: *mut c_void, flags: u32) -> i32;
    fn BCryptHash(
        algorithm: *mut c_void,
        secret: *const u8,
        secret_len: u32,
        input: *const u8,
        input_len: u32,
        output: *mut u8,
        output_len: u32,
    ) -> i32;
}
pub fn token() -> Result<String> {
    let mut bytes = [0u8; 16];
    let code = unsafe { BCryptGenRandom(ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) };
    if code < 0 {
        bail!("Windows random number generator failed; nothing was written");
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn sha256(data: &[u8]) -> Result<String> {
    let len = u32::try_from(data.len())?;
    let name: Vec<u16> = "SHA256\0".encode_utf16().collect();
    let mut handle = ptr::null_mut();
    let status = unsafe { BCryptOpenAlgorithmProvider(&mut handle, name.as_ptr(), ptr::null(), 0) };
    if status < 0 {
        bail!("Could not open Windows SHA-256 provider");
    }
    let mut output = [0u8; 32];
    let status = unsafe {
        BCryptHash(
            handle,
            ptr::null(),
            0,
            data.as_ptr(),
            len,
            output.as_mut_ptr(),
            32,
        )
    };
    unsafe {
        BCryptCloseAlgorithmProvider(handle, 0);
    }
    if status < 0 {
        bail!("Windows SHA-256 failed");
    }
    Ok(output.iter().map(|b| format!("{b:02x}")).collect())
}
#[cfg(test)]
mod tests {
    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            super::sha256(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn random_tokens_are_distinct() {
        assert_ne!(super::token().unwrap(), super::token().unwrap());
    }
}
