use thiserror::Error;

//Serial port settings
const BAUDRATE: u32 = 57600;

const MSG_OK: u8 = 0xFF;
const MSG_ERROR: u8 = 0xFE;
const MAX_DATA_SIZE: u8 = 64;

//Requests
const RQ_DISCARD: u8 = 0;
const RQ_TEST: u8 = 1;
const RQ_INFO: u8 = 2;
const RQ_INT_TEST: u8 = 3;
const RQ_SELF_TEST: u8 = 4;
const RQ_DIGITAL_WRITE_0: u8 = 5;
const RQ_DIGITAL_WRITE_1: u8 = 6;
const RQ_DIGITAL_READ_0: u8 = 7;
const RQ_DIGITAL_READ_1: u8 = 8;
const RQ_READ_DIP_SWITCH: u8 = 9;
const RQ_ANALOG_WRITE_0: u8 = 10;
const RQ_ANALOG_WRITE_1: u8 = 11;
const RQ_ANALOG_READ: u8 = 12;
const RQ_ADC_DAC_STROKE: u8 = 13;
const RQ_PWM_SET_FREQ: u8 = 14;
const RQ_PWM_SET_VALUE: u8 = 15;
//NO NO NO!!!
const RQ_SET_MEM_8: u8 = 16;
const RQ_GET_MEM_8: u8 = 17;
const RQ_SET_MEM_16: u8 = 18;
const RQ_GET_MEM_16: u8 = 19;
const RQ_COUNTER_OFFSET: u8 = 20;
const RQ_SERVO_ENABLE: u8 = 21;
const RQ_SERVO_DISABLE: u8 = 22;
const RQ_SERVO_SET_POS: u8 = 23;


#[derive(Debug, )]
#[repr(u8)]
pub enum AnalogPort{
    Analog0 = 0,
    Analog1 = 1,
}

#[derive(Debug, Error)]
pub enum B15FCommandError {
    #[error("board error responded with error")]
    B15FError,
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] serialport::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct B15F<P> where P: serialport::SerialPort {
    port: P,
}

impl<P> B15F<P> where P: serialport::SerialPort {
    pub fn new(port: P) -> B15F<P> {
        B15F {
            port
        }
    }

    /// Writes a digital value to a specified port.
    ///
    /// This function sends a request to the specified digital port to write a given value.
    /// The port number must be either 0 or 1, otherwise, the function will panic.
    /// The function writes the request and the value to the port, flushes the port to ensure the request is sent,
    /// then reads the response from the port.
    /// If the response is MSG_OK, the function returns Ok(()), otherwise, it returns a B15FCommandError::B15FError.
    ///
    /// # Arguments
    ///
    /// * `port` - A u8 representing the port number to write to. Must be either 0 or 1.
    /// * `value` - A u8 representing the value to write to the port.
    ///
    /// # Returns
    ///
    /// * `Result<(), B15FCommandError>` - On success, returns Ok(()). On failure, returns a B15FCommandError.
    ///
    /// # Panics
    ///
    /// * If the port number is not 0 or 1, the function will panic.
    ///
    /// # Errors
    ///
    /// * If there is an IO error when writing to or reading from the port, the function will return a B15FCommandError::IoError.
    /// * If the response from the port is not MSG_OK, the function will return a B15FCommandError::B15FError.
    pub fn digital_write(&mut self, port: u8, value: u8) -> Result<(), B15FCommandError> {
        let request = match port {
            0 => RQ_DIGITAL_WRITE_0,
            1 => RQ_DIGITAL_WRITE_1,
            _ => panic!("digital write port must be 0 or 1")
        };
        let data = [request, value];
        self.port.write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush()
            .map_err(B15FCommandError::IoError)?;

        let mut response = [0u8];
        self.port.read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = response[0];
        if response == MSG_OK {
            Ok(())
        } else {
            Err(B15FCommandError::B15FError)
        }
    }

    /// Reads the digital value from a specified port.
    ///
    /// This function sends a request to the specified digital port to read its current value.
    /// The port number must be either 0 or 1, otherwise, the function will panic.
    /// The function writes the request to the port, flushes the port to ensure the request is sent,
    /// then reads the response from the port.
    /// The response is a single byte, which is then reversed (as the device sends the bits in reverse order).
    ///
    /// # Arguments
    ///
    /// * `port` - A u8 representing the port number to read from. Must be either 0 or 1.
    ///
    /// # Returns
    ///
    /// * `Result<u8, B15FCommandError>` - On success, returns the read value as a u8. On failure, returns a B15FCommandError.
    ///
    /// # Panics
    ///
    /// * If the port number is not 0 or 1, the function will panic.
    ///
    /// # Errors
    ///
    /// * If there is an IO error when writing to or reading from the port, the function will return a B15FCommandError::IoError.
    pub fn digital_read(&mut self, port: u8) -> Result<u8, B15FCommandError> {
        let request = match port {
            0 => RQ_DIGITAL_READ_0,
            1 => RQ_DIGITAL_READ_1,
            _ => panic!("digital read port must be 0 or 1")
        };
        let data = [request];
        self.port.write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush()
            .map_err(B15FCommandError::IoError)?;
        let mut response = [0u8];
        self.port.read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = response[0].reverse_bits();
        Ok(response)
    }

    /// Writes an analog value to a specified port.
    ///
    /// This function sends a request to the specified analog port to write a given value.
    /// The port number must be either 0 or 1, otherwise, the function will panic.
    /// The value must be between 0 and 1023, otherwise, the function will panic.
    /// The function writes the request and the value to the port, flushes the port to ensure the request is sent,
    /// then reads the response from the port.
    /// If the response is MSG_OK, the function returns Ok(()), otherwise, it returns a B15FCommandError::B15FError.
    ///
    /// # Arguments
    ///
    /// * `port` - A u8 representing the port number to write to. Must be either 0 or 1.
    /// * `value` - A u16 representing the value to write to the port. Must be between 0 and 1023.
    ///
    /// # Returns
    ///
    /// * `Result<(), B15FCommandError>` - On success, returns Ok(()). On failure, returns a B15FCommandError.
    ///
    /// # Panics
    ///
    /// * If the port number is not 0 or 1, the function will panic.
    /// * If the value is not between 0 and 1023, the function will panic.
    ///
    /// # Errors
    ///
    /// * If there is an IO error when writing to or reading from the port, the function will return a B15FCommandError::IoError.
    /// * If the response from the port is not MSG_OK, the function will return a B15FCommandError::B15FError.
    pub fn analog_write(&mut self, port: AnalogPort, value: u16) -> Result<(), B15FCommandError> {
        let request = match port {
            AnalogPort::Analog0 => RQ_ANALOG_WRITE_0,
            AnalogPort::Analog1 => RQ_ANALOG_WRITE_1,
        };
        if value > 1023 {
            panic!("analog write value must be between 0 and 1023")
        }
        let data = [request, (value & 0xFF) as u8, (value >> 8) as u8];
        self.port.write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush()
            .map_err(B15FCommandError::IoError)?;

        let mut response = [0u8];
        self.port.read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = response[0];
        if response == MSG_OK {
            Ok(())
        } else {
            Err(B15FCommandError::B15FError)
        }
    }

    /// Reads the analog value from a specified port.
    ///
    /// This function sends a request to the specified analog port to read its current value.
    /// The port number must be between 0 and 7, otherwise, the function will panic.
    /// The function writes the request to the port, flushes the port to ensure the request is sent,
    /// then reads the response from the port.
    /// The response is a two-byte value, which is then converted to a u16 using little-endian byte order.
    ///
    /// # Arguments
    ///
    /// * `port` - A u8 representing the port number to read from. Must be between 0 and 7.
    ///
    /// # Returns
    ///
    /// * `Result<u16, B15FCommandError>` - On success, returns the read value as a u16. On failure, returns a B15FCommandError.
    ///
    /// # Panics
    ///
    /// * If the port number is not between 0 and 7, the function will panic.
    ///
    /// # Errors
    ///
    /// * If there is an IO error when writing to or reading from the port, the function will return a B15FCommandError::IoError.
    pub fn analog_read(&mut self, port: u8) -> Result<u16, B15FCommandError> {
        let request = RQ_ANALOG_READ;
        if port > 7 {
            panic!("analog read port must be between 0 and 7")
        }
        let data = [request, port];
        self.port.write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush()
            .map_err(B15FCommandError::IoError)?;
        let mut response = [0u8; 2];
        self.port.read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = u16::from_le_bytes(response);
        Ok(response)
    }



}