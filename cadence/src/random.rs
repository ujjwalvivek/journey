#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lfsr {
    state: u16,
}

impl Lfsr {
    pub const fn new(seed: u16) -> Self {
        Self {
            state: if seed == 0 { 0xACE1 } else { seed },
        }
    }

    pub const fn state(&self) -> u16 {
        self.state
    }

    pub fn next_u16(&mut self) -> u16 {
        let bit = self.state & 1;
        self.state >>= 1;
        if bit == 1 {
            self.state ^= 0xB400;
        }
        self.state
    }
}

impl Default for Lfsr {
    fn default() -> Self {
        Self::new(0xACE1)
    }
}
