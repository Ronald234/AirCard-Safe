use std::io::{self, Read};

const MAX_FRAME: usize = 16 * 1024 * 1024;

// The private Apple API exposes a signed count, without a verified Windows
// secure-connection error contract. This marker records no claimed cause.
#[derive(Debug)]
pub(super) struct ReceiveUnavailable;

impl std::fmt::Display for ReceiveUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Native log receive returned a negative result")
    }
}

impl std::error::Error for ReceiveUnavailable {}

pub(super) fn should_retry_receive(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    ) || error
        .get_ref()
        .is_some_and(|cause| cause.is::<ReceiveUnavailable>())
}

pub(super) fn read_frame<R: Read>(
    reader: &mut R,
    keep_reading: &mut impl FnMut() -> bool,
) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut header = [0; 5];
    if !read_exact_cancellable(reader, &mut header, keep_reading)? {
        return Ok(None);
    }
    let bytes: [u8; 4] = header[1..].try_into().unwrap();
    let length = match header[0] {
        1 => u32::from_be_bytes(bytes),
        2 => u32::from_le_bytes(bytes),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported log frame",
            ));
        }
    } as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid log frame length",
        ));
    }
    let mut payload = vec![0; length];
    if !read_exact_cancellable(reader, &mut payload, keep_reading)? {
        return Ok(None);
    }
    Ok(Some((header[0], payload)))
}

fn read_exact_cancellable<R: Read>(
    reader: &mut R,
    mut out: &mut [u8],
    keep_reading: &mut impl FnMut() -> bool,
) -> io::Result<bool> {
    while !out.is_empty() {
        if !keep_reading() {
            return Ok(false);
        }
        match reader.read(out) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) if count <= out.len() => {
                out = &mut out[count..];
            }
            Ok(_) => return Err(io::ErrorKind::InvalidData.into()),
            Err(error) if should_retry_receive(&error) => {
                // A timed-out fragment is retained, not discarded as a new header.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}

pub(super) fn record_line(record: &[u8]) -> Option<String> {
    if record.len() < 129 || record[0] != 2 {
        return None;
    }
    let header = u32::from_le_bytes(record[5..9].try_into().ok()?) as usize;
    let process = u16::from_le_bytes(record[37..39].try_into().ok()?) as usize;
    let image = u16::from_le_bytes(record[107..109].try_into().ok()?) as usize;
    let message = u32::from_le_bytes(record[109..113].try_into().ok()?) as usize;
    // No Wallet discovery event needs unbounded text. Ignore oversized records.
    if header < 129 || header > record.len() || process == 0 || message == 0 || message > 64 * 1024
    {
        return None;
    }
    let process_end = header.checked_add(process)?;
    let image_end = process_end.checked_add(image)?;
    let message_end = image_end.checked_add(message)?;
    if message_end > record.len() {
        return None;
    }
    let text = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .to_string()
    };
    let process_name = text(&record[header..process_end]);
    let image_name = text(&record[process_end..image_end]);
    let message = text(&record[image_end..message_end]).replace(['\r', '\n'], " ");
    Some(format!(
        "{}({}): {}",
        process_name.rsplit('/').next()?,
        image_name.rsplit('/').next()?,
        message
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // The native service may split every header/payload across separate reads.
    struct Fragmented(Cursor<Vec<u8>>);
    impl Read for Fragmented {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let take = out.len().min(2);
            self.0.read(&mut out[..take])
        }
    }

    #[test]
    fn fragmented_frames_preserve_payload_and_mixed_endianness() {
        let mut reader = Fragmented(Cursor::new(vec![
            1, 0, 0, 0, 3, 7, 8, 9, 2, 2, 0, 0, 0, 10, 11,
        ]));
        assert_eq!(
            read_frame(&mut reader, &mut || true).unwrap(),
            Some((1, vec![7, 8, 9]))
        );
        assert_eq!(
            read_frame(&mut reader, &mut || true).unwrap(),
            Some((2, vec![10, 11]))
        );
    }

    #[test]
    fn malformed_frame_lengths_are_rejected_before_payload_read() {
        for data in [
            vec![2, 0, 0, 0, 0],
            vec![2, 1, 0, 0, 1],
            vec![3, 1, 0, 0, 0],
        ] {
            assert!(read_frame(&mut Cursor::new(data), &mut || true).is_err());
        }
    }

    #[test]
    fn truncated_frames_return_error_and_stopped_scan_does_not_read() {
        assert!(read_frame(&mut Cursor::new(vec![2, 3, 0, 0, 0, 1]), &mut || true).is_err());
        assert_eq!(
            read_frame(&mut Cursor::new(vec![255]), &mut || false).unwrap(),
            None
        );
    }

    fn record() -> Vec<u8> {
        let mut data = vec![0; 129];
        data[0] = 2;
        data[5..9].copy_from_slice(&129u32.to_le_bytes());
        data[37..39].copy_from_slice(&6u16.to_le_bytes());
        data[107..109].copy_from_slice(&4u16.to_le_bytes());
        let message = b"selected pass uniqueID :\n\"gym-membership-123456789\"\0";
        data[109..113].copy_from_slice(&(message.len() as u32).to_le_bytes());
        data.extend_from_slice(b"nfcd\0\0nfc\0");
        data.extend_from_slice(message);
        data
    }

    #[test]
    fn complete_multiline_record_keeps_process_context() {
        let line = record_line(&record()).unwrap();
        assert_eq!(
            line,
            "nfcd(nfc): selected pass uniqueID : \"gym-membership-123456789\""
        );
        assert_eq!(
            super::super::extract_wallet_items(&line),
            vec![super::super::WalletItem::CandidatePass(
                "gym-membership-123456789".into()
            )]
        );
    }

    #[test]
    fn incomplete_or_overflowing_record_is_ignored() {
        let valid = record();
        for length in [0, 1, 128, 129, valid.len() - 1] {
            assert!(record_line(&valid[..length]).is_none());
        }
        let mut invalid = valid.clone();
        invalid[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(record_line(&invalid).is_none());
        invalid = valid.clone();
        invalid[109..113].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(record_line(&invalid).is_none());
        invalid = valid;
        invalid[0] = 1;
        assert!(record_line(&invalid).is_none());
    }

    #[test]
    fn timeout_between_fragments_retains_frame_until_cancelled() {
        struct Timeouts {
            bytes: Cursor<Vec<u8>>,
            timeout: bool,
        }
        impl Read for Timeouts {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                self.timeout = !self.timeout;
                if self.timeout {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                let take = out.len().min(1);
                self.bytes.read(&mut out[..take])
            }
        }
        let mut r = Timeouts {
            bytes: Cursor::new(vec![2, 1, 0, 0, 0, 42]),
            timeout: false,
        };
        assert_eq!(
            read_frame(&mut r, &mut || true).unwrap(),
            Some((2, vec![42]))
        );
        let mut always_timeout = Timeouts {
            bytes: Cursor::new(Vec::new()),
            timeout: false,
        };
        let mut checks = 0;
        assert_eq!(
            read_frame(&mut always_timeout, &mut || {
                checks += 1;
                checks < 2
            })
            .unwrap(),
            None
        );
    }

    #[test]
    fn opaque_native_negative_result_is_cancelled_without_busy_looping() {
        struct Unavailable;
        impl Read for Unavailable {
            fn read(&mut self, _out: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other(ReceiveUnavailable))
            }
        }
        let started = std::time::Instant::now();
        let mut checks = 0;
        assert_eq!(
            read_frame(&mut Unavailable, &mut || {
                checks += 1;
                checks < 4
            })
            .unwrap(),
            None
        );
        // Each failed receive yields; a persistent error cannot spin at full speed.
        assert!(started.elapsed() >= std::time::Duration::from_millis(20));
    }
}
