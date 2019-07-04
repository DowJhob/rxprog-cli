use std::convert::Infallible;
use std::io;
use std::marker::PhantomData;
use std::num::Wrapping;
use std::str;

mod boot_program_status_inquiry;
mod clock_mode_inquiry;
mod clock_mode_selection;
mod device_selection;
mod erasure_block_information_inquiry;
mod multiplication_ratio_inquiry;
mod new_bit_rate_selection;
mod new_bit_rate_selection_confirmation;
mod operating_frequency_inquiry;
mod programming_erasure_state_transition;
mod programming_size_inquiry;
mod supported_device_inquiry;
mod user_area_information_inquiry;
mod user_boot_area_information_inquiry;

trait Command {
    type Response;
    type Error;

    fn execute<T: io::Read + io::Write>(
        &self,
        p: &mut T,
    ) -> io::Result<Result<Self::Response, Self::Error>>;
}

trait Transmit {
    fn tx<T: io::Write>(&self, p: &mut T) -> io::Result<()>;
}

struct CommandData {
    opcode: u8,
    has_size_field: bool,
    payload: Vec<u8>,
}

impl CommandData {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        let payload = &self.payload;
        let payload_size = payload.len();

        bytes.push(self.opcode);

        if self.has_size_field {
            bytes.push(payload_size as u8);
        }

        bytes.extend(payload);

        if payload_size != 0 {
            let sum = bytes.iter().map(|x| Wrapping(*x)).sum::<Wrapping<u8>>().0;
            let checksum = !sum + 1;
            bytes.push(checksum);
        }

        bytes
    }
}

trait TransmitCommandData {
    fn command_data(&self) -> CommandData;
}

impl<T: TransmitCommandData> Transmit for T {
    fn tx<U: io::Write>(&self, p: &mut U) -> io::Result<()> {
        p.write(&self.command_data().bytes())?;
        p.flush()?;

        Ok(())
    }
}

trait Receive {
    type Response;
    type Error;

    fn rx<T: io::Read>(&self, p: &mut T) -> io::Result<Result<Self::Response, Self::Error>>;
}

impl<T: Transmit + Receive> Command for T {
    type Response = T::Response;
    type Error = T::Error;

    fn execute<U: io::Read + io::Write>(
        &self,
        p: &mut U,
    ) -> io::Result<Result<Self::Response, Self::Error>> {
        self.tx(p)?;
        Ok(self.rx(p)?)
    }
}

enum SimpleResponse {
    Response(u8),
    Error(u8),
}

enum SizedResponse {
    Response(Vec<u8>),
    Error(u8),
}

enum ResponseFirstByte {
    Byte(u8),
    OneByteOf(Vec<u8>),
}

enum ErrorResponseFirstByte {
    None,
    Byte(u8),
}

struct ResponseReader<T: io::Read, U> {
    p: T,
    response_first_byte: ResponseFirstByte,
    error_response_first_byte: ErrorResponseFirstByte,

    phantom: PhantomData<U>,
}

impl<T: io::Read, U> ResponseReader<T, U> {
    fn new(
        p: T,
        response_first_byte: ResponseFirstByte,
        error_response_first_byte: ErrorResponseFirstByte,
    ) -> ResponseReader<T, U> {
        ResponseReader {
            p: p,
            response_first_byte: response_first_byte,
            error_response_first_byte: error_response_first_byte,

            phantom: PhantomData,
        }
    }

    fn read_header(&mut self) -> io::Result<Result<u8, u8>> {
        let mut first_byte = [0u8; 1];
        self.p.read_exact(&mut first_byte)?;
        let first_byte = first_byte[0];

        if let ErrorResponseFirstByte::Byte(error_response_first_byte) =
            self.error_response_first_byte
        {
            if first_byte == error_response_first_byte {
                let mut error = [0u8; 1];
                self.p.read_exact(&mut error)?;
                let error = error[0];

                return Ok(Err(error));
            }
        }

        let is_valid_response_first_byte = match &self.response_first_byte {
            ResponseFirstByte::Byte(response_first_byte) => first_byte == *response_first_byte,
            ResponseFirstByte::OneByteOf(response_first_bytes) => response_first_bytes
                .iter()
                .find(|&&x| x == first_byte)
                .is_some(),
        };

        assert!(
            is_valid_response_first_byte,
            "Response did not start with a valid byte"
        );

        Ok(Ok(first_byte))
    }
}

impl<T: io::Read> ResponseReader<T, SimpleResponse> {
    fn read_response(mut self) -> io::Result<SimpleResponse> {
        match self.read_header()? {
            Ok(first_byte) => Ok(SimpleResponse::Response(first_byte)),
            Err(error) => Ok(SimpleResponse::Error(error)),
        }
    }
}

impl<T: io::Read> ResponseReader<T, SizedResponse> {
    fn read_response(mut self) -> io::Result<SizedResponse> {
        let header = self.read_header()?;

        if let Err(error) = header {
            return Ok(SizedResponse::Error(error));
        }

        let mut size = [0u8; 1];
        self.p.read_exact(&mut size)?;
        let size = size[0];

        let mut data = vec![0u8; size as usize];
        self.p.read_exact(&mut data)?;

        // TODO: Check checksum
        let mut _checksum = [0u8; 1];
        self.p.read_exact(&mut _checksum)?;
        let _checksum = _checksum[0];

        Ok(SizedResponse::Response(data))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MultiplicationRatio {
    DivideBy(u8),
    MultiplyBy(u8),
}

impl From<u8> for MultiplicationRatio {
    fn from(item: u8) -> Self {
        let item_signed = i8::from_le_bytes([item]);
        let ratio = item_signed.abs() as u8;

        match item_signed {
            x if x < 0 => MultiplicationRatio::DivideBy(ratio),
            x if x > 0 => MultiplicationRatio::MultiplyBy(ratio),
            _ => panic!("Multiplication ratio cannot be zero"),
        }
    }
}

impl From<MultiplicationRatio> for u8 {
    fn from(item: MultiplicationRatio) -> Self {
        match item {
            MultiplicationRatio::DivideBy(ratio) => -(ratio as i8) as u8,
            MultiplicationRatio::MultiplyBy(ratio) => ratio as u8,
        }
    }
}

fn all_read<T: io::Read>(p: &mut T) -> bool {
    let mut buf = [0u8; 1];
    p.read(&mut buf).unwrap() == 0
}
