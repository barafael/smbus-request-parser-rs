#![no_std]

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests;

pub trait CommandHandler {
    type Error;

    fn handle_read_byte(&self) -> Option<u8>;
    fn handle_read_byte_data(&self, reg: u8) -> Option<u8>;
    fn handle_read_word_data(&self, reg: u8) -> Option<u16>;
    fn handle_read_block_data(&self, reg: u8, index: u8) -> Option<u8>;

    fn handle_write_byte(&mut self, data: u8) -> Result<(), Self::Error>;
    fn handle_write_byte_data(&mut self, reg: u8, data: u8) -> Result<(), Self::Error>;
    fn handle_write_word_data(&mut self, reg: u8, data: u16) -> Result<(), Self::Error>;
    fn handle_write_block_data(
        &mut self,
        reg: u8,
        count: u8,
        block: &[u8],
    ) -> Result<(), Self::Error>;

    fn handle_i2c_event(
        &mut self,
        event: &mut I2CEvent,
        mut bus_state: &mut SMBusState,
    ) -> Result<(), SMBusProtocolError> {
        match event {
            I2CEvent::Initiated { direction } => bus_state.direction = Some(*direction),
            I2CEvent::ReceivedByte { byte } => {
                if bus_state.index >= RECEIVE_BUFFER_SIZE {
                    let err = Err(SMBusProtocolError::InvalidWriteBound(bus_state.index - 2));
                    *bus_state = SMBusState::default();
                    return err;
                }
                bus_state.received_data[bus_state.index as usize] = *byte;
                bus_state.index += 1;
            }
            I2CEvent::RequestedByte { byte } => {
                if bus_state.direction != Some(Direction::SlaveToMaster) {
                    return Err(SMBusProtocolError::WrongDirection(bus_state.direction));
                }
                match bus_state.index {
                    0 => {
                        if bus_state.direction == Some(Direction::SlaveToMaster) {
                            if let Some(data) = self.handle_read_byte() {
                                **byte = data;
                            } else {
                                return Err(SMBusProtocolError::ReadByteUnsupported);
                            }
                        } else {
                            return Err(SMBusProtocolError::WrongDirection(bus_state.direction));
                        }
                    }
                    1 => {
                        let register = bus_state.received_data[0];
                        if let Some(data) = self.handle_read_byte_data(register) {
                            bus_state.current_transfer = Some(StatefulTransfer::Byte(data));
                            **byte = data;
                        } else if let Some(data) = self.handle_read_word_data(register) {
                            bus_state.current_transfer = Some(StatefulTransfer::Word(data));
                            **byte = data as u8;
                        } else if let Some(data) = self.handle_read_block_data(register, 0) {
                            bus_state.current_transfer = Some(StatefulTransfer::Block(data));
                            **byte = data;
                        } else {
                            return Err(SMBusProtocolError::InvalidReadRegister(register));
                        }
                    }
                    2 => {
                        let first_byte = bus_state.received_data[0];
                        match bus_state.current_transfer {
                            Some(StatefulTransfer::Byte(_)) => {}
                            Some(StatefulTransfer::Word(data)) => {
                                **byte = (data >> 8) as u8;
                                bus_state.current_transfer = None;
                            }
                            Some(StatefulTransfer::Block(_)) => {
                                if let Some(data) = self.handle_read_block_data(first_byte, 1) {
                                    **byte = data;
                                } else {
                                    return Err(SMBusProtocolError::InvalidReadBound(1));
                                }
                            }
                            _ => return Err(SMBusProtocolError::InvalidReadBound(2)),
                        }
                    }
                    n => {
                        if let Some(StatefulTransfer::Block(_)) = bus_state.current_transfer {
                            if let Some(data) =
                                self.handle_read_block_data(bus_state.received_data[0], n - 1)
                            {
                                **byte = data;
                            }
                        }
                    }
                }
                bus_state.index += 1;
            }
            I2CEvent::Stopped => {
                if bus_state.direction == Some(Direction::MasterToSlave) {
                    match bus_state.index {
                        0 => return Err(SMBusProtocolError::QuickCommandUnsupported),
                        1 => {
                            if let Err(_e) = self.handle_write_byte(bus_state.received_data[0]) {
                                return Err(SMBusProtocolError::WriteByteUnsupported);
                            }
                        }
                        2 => {
                            if let Err(_e) = self.handle_write_byte_data(
                                bus_state.received_data[0],
                                bus_state.received_data[1],
                            ) {
                                return Err(SMBusProtocolError::InvalidWriteRegister(
                                    bus_state.received_data[0],
                                ));
                            }
                        }
                        3 => {
                            let data: u16 = bus_state.received_data[1] as u16
                                | (bus_state.received_data[2] as u16) << 8;
                            if let Err(_e) =
                                self.handle_write_word_data(bus_state.received_data[0], data)
                            {
                                return Err(SMBusProtocolError::InvalidWriteRegister(
                                    bus_state.received_data[0],
                                ));
                            };
                        }
                        4..=RECEIVE_BUFFER_SIZE => {
                            let reg = bus_state.received_data[0];
                            let count = bus_state.received_data[1];
                            if count > 32 {
                                return Err(SMBusProtocolError::InvalidWriteBlockSize(count));
                            }
                            let slice = &bus_state.received_data[2usize..count as usize + 2];
                            if let Err(_e) = self.handle_write_block_data(reg, count, slice) {
                                return Err(SMBusProtocolError::InvalidWriteBound(count));
                            }
                        }
                        n => return Err(SMBusProtocolError::InvalidWriteBound(n)),
                    };
                }
                bus_state.received_data.iter_mut().for_each(|x| *x = 0);
                bus_state.index = 0;
                bus_state.direction = None;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    MasterToSlave,
    SlaveToMaster,
}

#[derive(Debug)]
pub enum I2CEvent<'a> {
    Initiated { direction: Direction },
    ReceivedByte { byte: u8 },
    RequestedByte { byte: &'a mut u8 },
    Stopped,
}

#[derive(Debug, PartialEq, Eq)]
enum StatefulTransfer {
    Byte(u8),
    Word(u16),
    Block(u8),
}

const RECEIVE_BUFFER_SIZE: u8 = 34;

#[derive(Debug)]
pub struct SMBusState {
    index: u8,
    received_data: [u8; RECEIVE_BUFFER_SIZE as usize],
    direction: Option<Direction>,
    current_transfer: Option<StatefulTransfer>,
}

impl Default for SMBusState {
    fn default() -> Self {
        Self {
            index: 0,
            received_data: [0; RECEIVE_BUFFER_SIZE as usize],
            direction: None,
            current_transfer: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum SMBusProtocolError {
    WrongDirection(Option<Direction>),
    QuickCommandUnsupported,
    UnsupportedBlockLength(u8),
    ReadByteUnsupported,
    WriteByteUnsupported,
    InvalidWriteBound(u8),
    InvalidReadBound(u8),
    InvalidWriteBlockSize(u8),
    InvalidReadBlockSize(u8),
    InvalidReadRegister(u8),
    InvalidWriteRegister(u8),
}
