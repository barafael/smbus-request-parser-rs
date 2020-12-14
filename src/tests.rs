use crate::*;

#[test]
fn test_read_byte() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    state.some_byte = 0xa2;

    let mut event = I2CEvent::Addr {
        direction: Direction::SlaveToMaster,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0xa2, data);
}

#[test]
fn test_write_byte() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteByteCommand::IncrementByte as u8,
    };
    let mut stop = I2CEvent::Stop;

    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(2, state.some_byte);

    event = I2CEvent::ReceivedByte {
        byte: WriteByteCommand::ResetByte as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(0, state.some_byte);
}

#[test]
fn test_write_byte_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteByteDataCommand::SetByteA as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    //std::dbg!(bus_state.index);

    event = I2CEvent::ReceivedByte { byte: 0x42 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(0x42, state.byte_a);
}

#[test]
fn test_write_word_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteWordDataCommand::SetWordAB as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0x42 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xdc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(0x42, state.byte_a);
    assert_eq!(0xdc, state.byte_b);
}

#[test]
fn test_read_byte_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    state.byte_b = 0x32;

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: ReadByteDataCommand::GetByteB as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut restart = I2CEvent::Addr {
        direction: Direction::SlaveToMaster,
    };
    state
        .handle_i2c_event(&mut restart, &mut bus_state)
        .unwrap();

    let mut data: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    assert_eq!(0x32, data);
}

#[test]
fn test_read_word_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    state.byte_a = 0x42;
    state.byte_b = 0x32;

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: ReadWordDataCommand::GetWordAB as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut restart = I2CEvent::Addr {
        direction: Direction::SlaveToMaster,
    };
    state
        .handle_i2c_event(&mut restart, &mut bus_state)
        .unwrap();

    let mut data1: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data1 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data2: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data2 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(0x42, data1);
    assert_eq!(0x32, data2);
}

#[test]
fn test_write_block_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteBlockDataCommand::SetBlockABC as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 3 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xac };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xbc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xcc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(0xac, state.byte_a);
    assert_eq!(0xbc, state.byte_b);
    assert_eq!(0xcc, state.byte_c);
}

#[test]
fn test_read_block_data() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    state.byte_a = 0xa2;
    state.byte_b = 0xb2;
    state.byte_c = 0xc2;

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: ReadBlockDataCommand::GetBlockABC as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut restart = I2CEvent::Addr {
        direction: Direction::SlaveToMaster,
    };
    state
        .handle_i2c_event(&mut restart, &mut bus_state)
        .unwrap();

    let mut count: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut count };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data1: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data1 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data2: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data2 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data3: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data3 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    state.handle_i2c_event(&mut stop, &mut bus_state).unwrap();

    assert_eq!(3, count);
    assert_eq!(0xa2, data1);
    assert_eq!(0xb2, data2);
    assert_eq!(0xc2, data3);
}

#[test]
fn test_read_block_data_too_many_reads() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    state.byte_a = 0xf2;
    state.byte_b = 0xf2;
    state.byte_c = 0xf2;

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: ReadBlockDataCommand::GetBlockABC as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut restart = I2CEvent::Addr {
        direction: Direction::SlaveToMaster,
    };
    state
        .handle_i2c_event(&mut restart, &mut bus_state)
        .unwrap();

    let mut count: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut count };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data1: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data1 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data2: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data2 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data3: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data3 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut data4: u8 = 0;
    event = I2CEvent::RequestedByte { byte: &mut data4 };
    let error = state.handle_i2c_event(&mut event, &mut bus_state);

    assert_eq!(3, count);
    assert_eq!(0xf2, data1);
    assert_eq!(0xf2, data2);
    assert_eq!(0xf2, data3);
    assert_eq!(SMBusProtocolError::InvalidReadBound(5), error.err().unwrap())
}

#[test]
fn test_write_block_data_too_few_reads() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteBlockDataCommand::SetBlockABC as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 3 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xac };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xbc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    let error = state.handle_i2c_event(&mut stop, &mut bus_state).err().unwrap();

    assert_eq!(0x00, state.byte_a);
    assert_eq!(0x00, state.byte_b);
    assert_eq!(0x00, state.byte_c);
    assert_eq!(SMBusProtocolError::InvalidWriteBound(2), error);
}

#[test]
fn test_write_block_data_invalid_read_size() {
    let mut state = State::default();
    let mut bus_state = SMBusState::default();

    let mut event = I2CEvent::Addr {
        direction: Direction::MasterToSlave,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte {
        byte: WriteBlockDataCommand::SetBlockABC as u8,
    };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 5 };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xac };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xbc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xcc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xdc };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    event = I2CEvent::ReceivedByte { byte: 0xec };
    state.handle_i2c_event(&mut event, &mut bus_state).unwrap();

    let mut stop = I2CEvent::Stop;
    let error = state.handle_i2c_event(&mut stop, &mut bus_state).err().unwrap();

    assert_eq!(0x00, state.byte_a);
    assert_eq!(0x00, state.byte_b);
    assert_eq!(0x00, state.byte_c);
    assert_eq!(SMBusProtocolError::InvalidWriteBlockSize(5), error);
}
