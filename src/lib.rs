#![no_std]

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests;

trait CommandHandler {
    fn handle_read_byte(&self) -> Option<u8>;
    fn handle_read_byte_data(&self, reg: u8) -> Option<u8>;
    fn handle_read_word_data(&self, reg: u8) -> Option<u16>;
    fn handle_read_block_data(&self, reg: u8, index: u8) -> Option<u8>;

    fn handle_write_byte(&mut self, data: u8) -> Result<(), ()>;
    fn handle_write_byte_data(&mut self, reg: u8, data: u8) -> Result<(), ()>;
    fn handle_write_word_data(&mut self, reg: u8, data: u16) -> Result<(), ()>;
    fn handle_write_block_data(&mut self, reg: u8, count: u8, block: [u8; 32]) -> Result<(), ()>;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    MasterToSlave,
    SlaveToMaster,
}

pub enum I2CEvent<'a> {
    Initiated { direction: Direction },
    ReceivedByte { byte: u8 },
    RequestedByte { byte: &'a mut u8 },
    Stopped,
}

#[derive(Debug)]
enum StatefulTransfer {
    ReadWord(u16),
    ReadBlock(u8, [u8; 32]),

    WriteWord(u16),
    WriteBlock(u8, [u8; 32]),
}

#[derive(Default, Debug)]
pub struct SMBusState {
    index: u8,
    received_data: [u8; 32],
    direction: Option<Direction>,
    current_transfer: Option<StatefulTransfer>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum SMBusProtocolError {
    WrongDirection(Option<Direction>),
    QuickCommandUnsupported,
    UnsupportedBlockLength,
    ReadByteUnsupported,
    WriteByteUnsupported,
    InvalidWriteBound(u8),
    InvalidReadBound(u8),
    InvalidWriteBlockSize(u8),
    InvalidReadBlockSize(u8),
    InvalidReadRegister(u8),
    InvalidWriteRegister(u8),
}

impl dyn CommandHandler {
    pub fn handle_i2c_event(
        &mut self,
        event: &mut I2CEvent,
        bus_state: &mut SMBusState,
    ) -> Result<(), SMBusProtocolError> {
        match event {
            I2CEvent::Initiated { direction } => bus_state.direction = Some(*direction),
            I2CEvent::ReceivedByte { byte } => {
                bus_state.received_data[bus_state.index as usize] = *byte;
                bus_state.index += 1;
                if bus_state.index == 35 {
                    return Err(SMBusProtocolError::UnsupportedBlockLength);
                }
            }
            I2CEvent::RequestedByte { byte } => {
                match bus_state.index {
                    0 => {
                        if let Some(Direction::SlaveToMaster) = bus_state.direction {
                            if let Some(data) = self.handle_read_byte() {
                                **byte = data;
                            } else {
                                return Err(SMBusProtocolError::ReadByteUnsupported);
                            }
                        } else {
                            return Err(SMBusProtocolError::WrongDirection(bus_state.direction));
                        }
                    },
                    n => {
                        let first_byte = bus_state.received_data[0];
                        if n == 1 {
                            if let Some(data) = self.handle_read_byte_data(first_byte) {
                                **byte = data;
                            }
                            if let Some(data) = self.handle_read_word_data(first_byte) {
                                bus_state.current_transfer = Some(StatefulTransfer::ReadWord(data));
                                **byte = data as u8;
                            } else {
                                return Err(SMBusProtocolError::InvalidReadRegister(first_byte));
                            }
                        } else if n == 2 {
                            match bus_state.current_transfer {
                                Some(StatefulTransfer::ReadWord(data)) => {
                                    **byte = (data >> 8) as u8;
                                    bus_state.current_transfer = None;
                                },
                                _ => return Err(SMBusProtocolError::InvalidReadBound(2)),
                            }
                        } else {
                            return Err(SMBusProtocolError::InvalidReadBound(2));
                        }
                    }
                }
                bus_state.index += 1;
            }
            I2CEvent::Stopped => {
                match bus_state.index {
                    0 => return Err(SMBusProtocolError::QuickCommandUnsupported),
                    1 => if let Err(()) = self.handle_write_byte(bus_state.received_data[0]) {
                        return Err(SMBusProtocolError::WriteByteUnsupported);
                    }
                    2 => if let Err(()) = self.handle_write_byte_data(bus_state.received_data[0], bus_state.received_data[1]) {
                        return Err(SMBusProtocolError::InvalidWriteRegister(bus_state.received_data[0]));
                    }
                    3 => {
                        let data: u16 = bus_state.received_data[1] as u16 | (bus_state.received_data[2] as u16) << 8;
                        if let Err(()) = self.handle_write_word_data(bus_state.received_data[0], data) {
                            return Err(SMBusProtocolError::InvalidWriteRegister(bus_state.received_data[0]));
                        };
                    }
                    4..=32 => {
                        // TODO increase buffer size to accommodate actual 32byte block transfers (right now register and block take a byte each)
                        let count = bus_state.received_data[1];
                    }
                    _ => unreachable!(),
                };
                bus_state.received_data.iter_mut().for_each(|x| *x = 0);
                bus_state.index = 0;
                bus_state.direction = None;
            }
        }
        return Ok(());
    }
}
