use super::*;
use std::io;

use super::reader::*;

#[derive(Debug)]
pub struct BlockErasure {
    pub block: u8,
}

impl TransmitCommandData for BlockErasure {
    fn command_data(&self) -> CommandData {
        CommandData {
            opcode: 0x58,
            has_size_field: true,
            payload: vec![self.block],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BlockErasureError {
    Checksum,
    BlockNumber,
    Erasure,
}

impl Receive for BlockErasure {
    type Response = ();
    type Error = BlockErasureError;

    fn rx<T: io::Read>(&self, p: &mut T) -> io::Result<Result<Self::Response, Self::Error>> {
        let mut reader = ResponseReader::<_, SimpleResponse, WithError>::new(
            p,
            ResponseFirstByte::Byte(0x06),
            ErrorFirstByte(0xD8),
        );

        let response = reader.read_response()?;

        Ok(match response {
            Ok(_) => Ok(()),
            Err(error_code) => Err(match error_code {
                0x11 => BlockErasureError::Checksum,
                0x29 => BlockErasureError::BlockNumber,
                0x51 => BlockErasureError::Erasure,
                _ => panic!("Unknown error code"),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx() -> io::Result<()> {
        let cmd = BlockErasure { block: 0x38 };
        let command_bytes = [0x58, 0x01, 0x38, 0x6F];
        let mut p = mockstream::MockStream::new();

        cmd.tx(&mut p)?;

        assert_eq!(p.pop_bytes_written(), command_bytes);

        Ok(())
    }

    #[test]
    fn test_rx_success() {
        let cmd = BlockErasure { block: 0x38 };
        let response_bytes = [0x06];
        let mut p = mockstream::MockStream::new();
        p.push_bytes_to_read(&response_bytes);

        let response = cmd.rx(&mut p).unwrap();

        assert_eq!(response, Ok(()));
        assert!(all_read(&mut p));
    }

    #[test]
    fn test_rx_fail() {
        let cmd = BlockErasure { block: 0x38 };
        let response_bytes = [0xD8, 0x29];
        let mut p = mockstream::MockStream::new();
        p.push_bytes_to_read(&response_bytes);

        let response = cmd.rx(&mut p).unwrap();

        assert_eq!(response, Err(BlockErasureError::BlockNumber));
        assert!(all_read(&mut p));
    }
}