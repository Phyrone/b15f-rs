#[cfg(feature = "experimental")]
use bitflags::bitflags;
#[cfg(feature = "log")]
use log::debug;
use rand::random;
use serialport::{SerialPortType};
#[cfg(windows)]
use serialport::{COMPort};
#[cfg(not(windows))]
use serialport::TTYPort;
use std::time::Duration;
use thiserror::Error;

#[cfg(windows)]
type NativePort = COMPort;
#[cfg(not(windows))]
type NativePort = TTYPort;

//Serial port settings
const BAUD: u32 = 57600;

const MSG_OK: u8 = 0xFF;
//const MSG_ERROR: u8 = 0xFE;
//const MAX_DATA_SIZE: u8 = 64;

//Requests
//const RQ_DISCARD: u8 = 0;
const RQ_TEST: u8 = 1;
//const RQ_INFO: u8 = 2;
//const RQ_INT_TEST: u8 = 3;
//const RQ_SELF_TEST: u8 = 4;
const RQ_DIGITAL_WRITE_0: u8 = 5;
const RQ_DIGITAL_WRITE_1: u8 = 6;
const RQ_DIGITAL_READ_0: u8 = 7;
const RQ_DIGITAL_READ_1: u8 = 8;
//const RQ_READ_DIP_SWITCH: u8 = 9;
const RQ_ANALOG_WRITE_0: u8 = 10;
const RQ_ANALOG_WRITE_1: u8 = 11;
const RQ_ANALOG_READ: u8 = 12;
//const RQ_ADC_DAC_STROKE: u8 = 13;
const RQ_PWM_SET_FREQ: u8 = 14;
const RQ_PWM_SET_VALUE: u8 = 15;
//NO NO NO!!!
//const RQ_SET_MEM_8: u8 = 16;
//const RQ_GET_MEM_8: u8 = 17;
//const RQ_SET_MEM_16: u8 = 18;
//const RQ_GET_MEM_16: u8 = 19;
//const RQ_COUNTER_OFFSET: u8 = 20;
//const RQ_SERVO_ENABLE: u8 = 21;
//const RQ_SERVO_DISABLE: u8 = 22;
//const RQ_SERVO_SET_POS: u8 = 23;

#[derive(Debug, Copy, Clone)]
pub enum Port {
    Port0,
    Port1,
}

#[cfg(feature = "experimental")]
bitflags! {
    pub struct ReadManyPorts: u16 {
        //Digital read 1-2
        const Digital0 = 0b0000_0001 << 8;
        const Digital1 = 0b0000_0010 << 8;

        //Analog read 1-8
        const ANALOG0 = 0b0000_0100;
        const ANALOG1 = 0b0000_1000;
        const ANALOG2 = 0b0001_0000;
        const ANALOG3 = 0b0010_0000;
        const ANALOG4 = 0b0100_0000;
        const ANALOG5 = 0b1000_0000;
        const ANALOG6 = 0b0000_0001;
        const ANALOG7 = 0b0000_0010;
    }
}

#[cfg(feature = "experimental")]
impl ReadManyPorts {
    pub fn from_analog(port: u8) -> Self {
        match port {
            0 => ReadManyPorts::ANALOG0,
            1 => ReadManyPorts::ANALOG1,
            2 => ReadManyPorts::ANALOG2,
            3 => ReadManyPorts::ANALOG3,
            4 => ReadManyPorts::ANALOG4,
            5 => ReadManyPorts::ANALOG5,
            6 => ReadManyPorts::ANALOG6,
            7 => ReadManyPorts::ANALOG7,
            _ => panic!("invalid analog port"),
        }
    }
    
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

#[derive(Debug, Error)]
pub enum B15FInitError {
    #[error("command error: {0}")]
    CommandError(#[from] B15FCommandError),
    #[error("device not found")]
    DeviceNotFound,
    #[error("device not supported")]
    DeviceNotSupported,
    #[error("Serial port error: {0}")]
    SerialPortError(#[from] serialport::Error),
}

pub struct B15F<P>
where
    P: serialport::SerialPort,
{
    port: P,
}

impl B15F<NativePort> {
    pub fn open_port(port_name: &str) -> Result<B15F<NativePort>, B15FInitError> {
        let port = serialport::new(port_name, BAUD)
            .timeout(Duration::from_millis(5000))
            .open_native()
            .map_err(B15FInitError::SerialPortError)?;
        B15F::from(port)
    }

    ///Automatically detects the B15F board and returns an instance of B15F.
    pub fn instance() -> Option<B15F<NativePort>> {
        let mut ports = serialport::available_ports().ok()?;
        ports.sort_unstable_by_key(port_priority);
        for port in ports {
            #[cfg(feature = "log")]
            debug!("[Discover] Check for B15 board on {}", port.port_name);
            let board = B15F::open_port(&port.port_name)
                .inspect_err(|err| {
                    #[cfg(feature = "log")]
                    debug!("[Discover] Failed to open {}: {}", port.port_name, err);
                })
                .ok()
                .and_then(|mut board| {
                    board
                        .test()
                        .inspect_err(|err| {
                            #[cfg(feature = "log")]
                            debug!("[Discover] Test failed for {}: {}", port.port_name, err);
                        })
                        .ok()?;
                    Some(board)
                });
            if let Some(board) = board {
                #[cfg(feature = "log")]
                debug!("[Discover] Choose B15 board on {}", port.port_name);
                return Some(board);
            }
        }
        None
    }
}

impl<P> B15F<P>
where
    P: serialport::SerialPort,
{
    pub fn from(port: P) -> Result<B15F<P>, B15FInitError> {
        let mut board = B15F { port };
        let pass = board.test()?;
        if !pass {
            return Err(B15FInitError::DeviceNotSupported);
        }
        Ok(board)
    }

    pub fn test(&mut self) -> Result<bool, B15FCommandError> {
        let rand = random::<u8>();
        let data = [RQ_TEST, rand];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        let mut response = [0u8; 2];
        self.port
            .read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        if response[0] != MSG_OK {
            return Err(B15FCommandError::B15FError);
        }
        let response = response[1];

        let pass = response == rand;
        Ok(pass)
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
    pub fn digital_write(&mut self, port: Port, value: u8) -> Result<(), B15FCommandError> {
        let request = match port {
            Port::Port0 => RQ_DIGITAL_WRITE_0,
            Port::Port1 => RQ_DIGITAL_WRITE_1,
        };
        let data = [request, value];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;

        let mut response = [0u8];
        self.port
            .read_exact(&mut response)
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
    pub fn digital_read(&mut self, port: Port) -> Result<u8, B15FCommandError> {
        self.send_digital_read_request(port)?;
        self.read_digital_response()
    }

    fn send_digital_read_request(&mut self, port: Port) -> Result<(), B15FCommandError> {
        let request = match port {
            Port::Port0 => RQ_DIGITAL_READ_0,
            Port::Port1 => RQ_DIGITAL_READ_1,
        };
        let data = [request];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;
        Ok(())
    }

    fn read_digital_response(&mut self) -> Result<u8, B15FCommandError> {
        let mut response = [0u8];
        self.port
            .read_exact(&mut response)
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
    pub fn analog_write(&mut self, port: Port, value: u16) -> Result<(), B15FCommandError> {
        let request = match port {
            Port::Port0 => RQ_ANALOG_WRITE_0,
            Port::Port1 => RQ_ANALOG_WRITE_1,
        };
        if value > 1023 {
            panic!("analog write value must be between 0 and 1023")
        }
        let data = [request, (value & 0xFF) as u8, (value >> 8) as u8];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;

        let mut response = [0u8];
        self.port
            .read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = response[0];
        if response == MSG_OK {
            Ok(())
        } else {
            Err(B15FCommandError::B15FError)
        }
    }

    /// This is an experimental function sending multiple read requests to the board before reading the response.
    /// It slightly reduces the latency compared to sending a single request per port.
    /// Depending on the b15 implementation, it may not work as expected (my b32 experimental board works fine).
    #[cfg(feature = "experimental")]
    pub fn experiment_read_many(
        &mut self,
        ports: ReadManyPorts,
    ) -> Result<([u8; 2], [u16; 7]), B15FCommandError> {
        
        if ports.contains(ReadManyPorts::Digital0) {
            self.send_digital_read_request(Port::Port0)?;
        }
        if ports.contains(ReadManyPorts::Digital1) {
            self.send_digital_read_request(Port::Port1)?;
        }
        for port in 0..8_u8 {
            let many_port = ReadManyPorts::from_analog(port);
            if ports.contains(many_port) {
                self.send_analog_read_request(port)?;
            }
        }
        self.port.flush()?;

        let mut digital = [0; 2];
        let mut analog = [0; 7];
        
        if ports.contains(ReadManyPorts::Digital0) {
            digital[0] = self.read_digital_response()?;
        }
        if ports.contains(ReadManyPorts::Digital1) {
            digital[1] = self.read_digital_response()?;
        }
        for port in 0..8_u8 {
            if ports.contains(ReadManyPorts::from_analog(port)) {
                analog[port as usize] = self.read_analog_response()?;
            }
        }

        Ok((digital, analog))
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
        self.send_analog_read_request(port)?;
        self.read_analog_response()
    }

    fn send_analog_read_request(&mut self, port: u8) -> Result<(), B15FCommandError> {
        assert!(port <= 7, "analog read port must be between 0 and 7");
        self.port
            .write_all(&[RQ_ANALOG_READ, port])
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;
        Ok(())
    }

    fn read_analog_response(&mut self) -> Result<u16, B15FCommandError> {
        let mut response = [0u8; 2];
        self.port
            .read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = u16::from_le_bytes(response);
        Ok(response)
    }

    pub fn set_pwm_frequency(&mut self, frequency: f32) -> Result<u8, B15FCommandError> {
        let data = frequency.to_le_bytes();
        let data = [RQ_PWM_SET_FREQ, data[0], data[1], data[2], data[3]];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;

        let mut response = [0u8];
        self.port
            .read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;

        let response = response[0];
        Ok(response)
    }

    pub fn set_pwm_vale(&mut self, value: u8) -> Result<(), B15FCommandError> {
        let data = [RQ_PWM_SET_VALUE, value];
        self.port
            .write_all(&data)
            .map_err(B15FCommandError::IoError)?;
        self.port.flush().map_err(B15FCommandError::IoError)?;
        let mut response = [0u8];
        self.port
            .read_exact(&mut response)
            .map_err(B15FCommandError::IoError)?;
        let response = response[0];
        if response == MSG_OK {
            Ok(())
        } else {
            Err(B15FCommandError::B15FError)
        }
    }
}

fn port_priority(port: &serialport::SerialPortInfo) -> u8 {
    let priority = match port.port_type {
        SerialPortType::UsbPort(_) => 0,
        SerialPortType::PciPort => 1,
        SerialPortType::BluetoothPort => 2,
        SerialPortType::Unknown => 3,
    };
    #[cfg(feature = "log")]
    debug!(
        "[Discover] Port priority: {} -> {}",
        port.port_name, priority
    );
    priority
}
