use super::*;
use std::io;

struct OperatingFrequencyInquiry {}

#[derive(Debug, PartialEq)]
struct OperatingFrequencyRange {
    minimum_frequency: u16,
    maximum_frequency: u16,
}

#[derive(Debug, PartialEq)]
struct OperatingFrequencyInquiryResponse {
    clock_types: Vec<OperatingFrequencyRange>,
}

impl TransmitCommandData for OperatingFrequencyInquiry {
    fn command_data(&self) -> CommandData {
        CommandData {
            opcode: 0x23,
            has_size_field: false,
            payload: vec![],
        }
    }
}

impl Receive for OperatingFrequencyInquiry {
    type Response = OperatingFrequencyInquiryResponse;
    type Error = Infallible;

    fn rx<T: io::Read>(&self, p: &mut T) -> io::Result<Result<Self::Response, Self::Error>> {
        let mut b1 = [0u8; 1];
        p.read_exact(&mut b1)?;
        let b1 = b1[0];

        assert_eq!(b1, 0x33);

        let mut _size = [0u8; 1];
        p.read_exact(&mut _size)?;

        let mut clock_type_count = [0u8; 1];
        p.read_exact(&mut clock_type_count)?;
        let clock_type_count = clock_type_count[0];

        let mut clock_types: Vec<OperatingFrequencyRange> = vec![];
        for _ in 0..clock_type_count {
            let mut minimum_frequency_bytes = [0u8; 2];
            p.read_exact(&mut minimum_frequency_bytes)?;

            let mut maximum_frequency_bytes = [0u8; 2];
            p.read_exact(&mut maximum_frequency_bytes)?;

            clock_types.push(OperatingFrequencyRange {
                // TODO: Check endianness
                minimum_frequency: u16::from_le_bytes(minimum_frequency_bytes),
                maximum_frequency: u16::from_le_bytes(maximum_frequency_bytes),
            });
        }

        Ok(Ok(OperatingFrequencyInquiryResponse {
            clock_types: clock_types,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx() -> io::Result<()> {
        let cmd = OperatingFrequencyInquiry {};
        let command_bytes = [0x23];
        let mut p = mockstream::MockStream::new();

        cmd.tx(&mut p)?;

        assert_eq!(p.pop_bytes_written(), command_bytes);

        Ok(())
    }

    #[test]
    fn test_rx() {
        let cmd = OperatingFrequencyInquiry {};
        let response_bytes = [
            0x33, 0x09, 0x02, 0xE8, 0x03, 0xD0, 0x07, 0x64, 0x00, 0x10, 0x27,
        ];
        let mut p = mockstream::MockStream::new();
        p.push_bytes_to_read(&response_bytes);

        let response = cmd.rx(&mut p).unwrap();

        assert_eq!(
            response,
            Ok(OperatingFrequencyInquiryResponse {
                clock_types: vec![
                    OperatingFrequencyRange {
                        minimum_frequency: 1000,
                        maximum_frequency: 2000
                    },
                    OperatingFrequencyRange {
                        minimum_frequency: 100,
                        maximum_frequency: 10000
                    },
                ],
            })
        );
        assert!(all_read(&mut p));
    }
}
