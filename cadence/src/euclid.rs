#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EuclideanPattern<const MAX_STEPS: usize> {
    pub pulses: usize,
    pub steps: usize,
    pub offset: usize,
    pattern: [bool; MAX_STEPS],
}

impl<const MAX_STEPS: usize> EuclideanPattern<MAX_STEPS> {
    pub const fn new(pulses: usize, steps: usize, offset: usize) -> Self {
        let steps = if steps > MAX_STEPS { MAX_STEPS } else { steps };
        let pulses = if pulses > steps { steps } else { pulses };
        let offset = if steps == 0 { 0 } else { offset % steps };
        let mut pattern = [false; MAX_STEPS];

        let mut i = 0;
        while i < steps {
            pattern[i] = pulses > 0 && (i * pulses) % steps < pulses;
            i += 1;
        }

        Self {
            pulses,
            steps,
            offset,
            pattern,
        }
    }

    pub const fn is_active(&self, step_index: usize) -> bool {
        if self.steps == 0 {
            false
        } else {
            self.pattern[(step_index + self.offset) % self.steps]
        }
    }

    pub const fn raw_step(&self, index: usize) -> bool {
        if index >= self.steps {
            false
        } else {
            self.pattern[index]
        }
    }

    pub fn active_count(&self) -> usize {
        let mut count = 0;
        for index in 0..self.steps {
            if self.pattern[index] {
                count += 1;
            }
        }
        count
    }
}
