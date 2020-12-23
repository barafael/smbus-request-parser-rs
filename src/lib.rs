#![no_std]

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests;

pub trait CommandHandler {
    fn handle_read_byte(&self) -> Option<u8>;
    fn handle_read_byte_data(&self, reg: u8) -> Option<u8>;
    fn handle_read_word_data(&self, reg: u8) -> Option<u16>;
    fn handle_read_block_data(&self, reg: u8, index: u8) -> Option<u8>;

    fn handle_write_byte(&mut self, data: u8) -> Result<(), ()>;
    fn handle_write_byte_data(&mut self, reg: u8, data: u8) -> Result<(), ()>;
    fn handle_write_word_data(&mut self, reg: u8, data: u16) -> Result<(), ()>;
    fn handle_write_block_data(&mut self, reg: u8, count: u8, block: &[u8]) -> Result<(), ()>;

    fn handle_i2c_event(
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
                if bus_state.direction != Some(Direction::SlaveToMaster) {
                    return Err(SMBusProtocolError::WrongDirection(bus_state.direction));
                }
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
                    }
                    1 => {
                        let first_byte = bus_state.received_data[0];
                        if let Some(data) = self.handle_read_byte_data(first_byte) {
                            **byte = data;
                        } else if let Some(data) = self.handle_read_word_data(first_byte) {
                            bus_state.current_transfer = Some(StatefulTransfer::ReadWord(data));
                            **byte = data as u8;
                        } else if let Some(data) = self.handle_read_block_data(first_byte, 0) {
                            bus_state.current_transfer = Some(StatefulTransfer::ReadBlock(data));
                            **byte = data;
                        } else {
                            return Err(SMBusProtocolError::InvalidReadRegister(first_byte));
                        }
                    }
                    2 => {
                        let first_byte = bus_state.received_data[0];
                        match bus_state.current_transfer {
                            Some(StatefulTransfer::ReadWord(data)) => {
                                **byte = (data >> 8) as u8;
                                bus_state.current_transfer = None;
                            }
                            Some(StatefulTransfer::ReadBlock(_)) => {
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
                        if let Some(StatefulTransfer::ReadBlock(_)) = bus_state.current_transfer {
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
                            if let Err(()) = self.handle_write_byte(bus_state.received_data[0]) {
                                return Err(SMBusProtocolError::WriteByteUnsupported);
                            }
                        }
                        2 => {
                            if let Err(()) = self.handle_write_byte_data(
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
                            if let Err(()) =
                                self.handle_write_word_data(bus_state.received_data[0], data)
                            {
                                return Err(SMBusProtocolError::InvalidWriteRegister(
                                    bus_state.received_data[0],
                                ));
                            };
                        }
                        4..=32 => {
                            // TODO increase buffer size to accommodate actual 32byte block transfers (right now register and block take a byte each)
                            let reg = bus_state.received_data[0];
                            let count = bus_state.received_data[1];
                            let slice = &bus_state.received_data[2usize..=count as usize + 2];
                            if let Err(()) = self.handle_write_block_data(reg, count, slice) {
                                return Err(SMBusProtocolError::InvalidWriteBound(count));
                            }
                        }
                        _ => unreachable!(),
                    };
                }
                bus_state.received_data.iter_mut().for_each(|x| *x = 0);
                bus_state.index = 0;
                bus_state.direction = None;
            }
        }
        return Ok(());
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

#[derive(Debug)]
enum StatefulTransfer {
    ReadWord(u16),
    ReadBlock(u8),
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
