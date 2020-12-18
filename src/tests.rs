use crate::*;

struct Thing {
    byte_a: u8,
    byte_b: u8,
    byte_c: u8,
}

impl CommandHandler for Thing {
    fn read_byte_supported(&self) -> bool {
        true
    }

    fn handle_read_byte(&self) -> Option<u8> {
        Some(self.byte_a)
    }

    fn handle_read_byte_data(&self, reg: u8) -> Option<u8> {
        match reg {
            1 => Some(self.byte_a),
            2 => Some(self.byte_b),
            3 => Some(self.byte_c),
            _ => None,
        }
    }

    fn handle_read_word_data(&self, reg: u8) -> Option<u16> {
        unimplemented!()
    }

    fn handle_read_block_data(&self, reg: u8, index: u8) -> Option<u8> {
        unimplemented!()
    }

    fn write_byte_supported(&self) -> bool {
        true
    }

    fn handle_write_byte(&mut self, data: u8) -> Result<(), ()> {
        self.byte_a = data;
        Ok(())
    }

    fn handle_write_byte_data(&mut self, reg: u8, data: u8) -> Result<(), ()> {
        unimplemented!()
    }

    fn handle_write_word_data(&mut self, reg: u8, data: u16) -> Result<(), ()> {
        unimplemented!()
    }

    fn handle_write_block_data(&mut self, reg: u8, count: u8, block: [u8; 32]) -> Result<(), ()> {
        unimplemented!()
    }
}

#[test]
fn test_read_byte() {
    let mut thing = Thing {
        byte_a: 0x42,
        byte_b: 0x10,
        byte_c: 0x20,
    };
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Initiated {
        direction: Direction::SlaveToMaster,
    };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data = 0;
    event = I2CEvent::RequestedByte { byte: &mut data };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::Stopped;
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0x42, data);
    assert_eq!(0x42, thing.byte_a);
    assert_eq!(0x10, thing.byte_b);
    assert_eq!(0x20, thing.byte_c);
}

#[test]
fn test_write_byte() {
    let mut thing = Thing {
        byte_a: 0x76,
        byte_b: 0x0a,
        byte_c: 0x0b,
    };
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Initiated {
        direction: Direction::MasterToSlave,
    };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0x76, thing.byte_a);

    event = I2CEvent::ReceivedByte { byte: 0x34 };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::Stopped;
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0x34, thing.byte_a);
    assert_eq!(0x0a, thing.byte_b);
    assert_eq!(0x0b, thing.byte_c);
}

#[test]
fn test_read_byte_data() {
    let mut thing = Thing {
        byte_a: 0x76,
        byte_b: 0x0a,
        byte_c: 0x0b,
    };
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Initiated {
        direction: Direction::MasterToSlave,
    };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 1 };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::Initiated {
        direction: Direction::SlaveToMaster,
    };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data = 0;
    event = I2CEvent::RequestedByte { byte: &mut data };
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::Stopped;
    thing.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0x76, data);
    assert_eq!(0x76, thing.byte_a);
    assert_eq!(0x0a, thing.byte_b);
    assert_eq!(0x0b, thing.byte_c);
}
