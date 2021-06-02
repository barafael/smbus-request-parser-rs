use rtt_target::rprintln;
use smbus_request_parser::*;

#[derive(Default, Debug)]
pub struct Data {
    byte_a: u8,
    byte_b: u8,
    byte_c: u8,

    count: usize,
    block: [u8; 32],
}

impl CommandHandler for Data {
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
        match reg {
            7 => {
                let data = self.byte_a as u16 | (self.byte_b as u16) << 8;
                Some(data)
            }
            8 => {
                let data = self.byte_b as u16 | (self.byte_c as u16) << 8;
                Some(data)
            }
            _ => None,
        }
    }

    fn handle_read_block_data(&self, reg: u8, index: u8) -> Option<u8> {
        rprintln!("block read {}", reg);
        match reg {
            11 => match index {
                0 => Some(8),
                //n => Some(n),
                1..=8 => Some(self.block[index as usize - 1]),
                _ => None,
            },
            12 => match index {
                0 => Some(16),
                1..=16 => Some(self.block[index as usize]),
                _ => None,
            },
            15 => {
                if index == 0 {
                    return Some(self.count as u8);
                } else {
                    return Some(self.block[index as usize]);
                }
            }
            _ => None,
        }
    }

    fn handle_write_byte(&mut self, data: u8) -> Result<(), ()> {
        self.byte_a = data;
        Ok(())
    }

    fn handle_write_byte_data(&mut self, reg: u8, data: u8) -> Result<(), ()> {
        match reg {
            4 => {
                self.byte_a = data;
                Ok(())
            }
            5 => {
                self.byte_b = data;
                Ok(())
            }
            6 => {
                self.byte_c = data;
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn handle_write_word_data(&mut self, reg: u8, data: u16) -> Result<(), ()> {
        match reg {
            9 => {
                let data1 = data as u8;
                let data2 = (data >> 8) as u8;
                self.byte_a = data1;
                self.byte_b = data2;
                Ok(())
            }
            10 => {
                let data1 = data as u8;
                let data2 = (data >> 8) as u8;
                self.byte_b = data1;
                self.byte_c = data2;
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn handle_write_block_data(&mut self, reg: u8, count: u8, block: &[u8]) -> Result<(), ()> {
        if count > 32 {
            return Err(());
        }
        match reg {
            13 => {
                if count != 20 {
                    return Err(());
                }
                for (index, value) in block.iter().take(count as usize).enumerate() {
                    self.block[index] = *value;
                }
                Ok(())
            }
            14 => {
                self.count = count as usize;
                for (index, value) in block.iter().take(count as usize).enumerate() {
                    self.block[index] = *value;
                }
                Ok(())
            }
            _ => Err(()),
        }
    }
}
