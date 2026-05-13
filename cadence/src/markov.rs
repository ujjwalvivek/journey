#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkovChain<const STATES: usize> {
    matrix: [[u8; STATES]; STATES],
    current_state: usize,
}

impl<const STATES: usize> MarkovChain<STATES> {
    pub const fn new(matrix: [[u8; STATES]; STATES], current_state: usize) -> Self {
        let current_state = if STATES == 0 {
            0
        } else if current_state >= STATES {
            STATES - 1
        } else {
            current_state
        };

        Self {
            matrix,
            current_state,
        }
    }

    pub const fn current_state(&self) -> usize {
        self.current_state
    }

    pub fn set_state(&mut self, state: usize) {
        if STATES > 0 {
            self.current_state = state.min(STATES - 1);
        }
    }

    pub fn next(&mut self, random_seed: u16) -> usize {
        if STATES == 0 {
            return 0;
        }

        let row = self.matrix[self.current_state];
        let mut total = 0u16;
        for weight in row {
            total = total.saturating_add(weight as u16);
        }

        if total == 0 {
            return self.current_state;
        }

        let mut threshold = random_seed % total;
        for (state, weight) in row.iter().enumerate() {
            let weight = *weight as u16;
            if threshold < weight {
                self.current_state = state;
                return state;
            }
            threshold -= weight;
        }

        self.current_state
    }
}
