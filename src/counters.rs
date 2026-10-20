// SPDX-License-Identifier: GPL-3.0-only

pub struct Counters {
    frames: Vec<Vec<(String, i32)>>,
}

impl Counters {
    pub fn new() -> Counters {
        Counters {
            frames: vec![Vec::new()],
        }
    }

    pub fn enter(&mut self) {
        self.frames.push(Vec::new());
    }

    pub fn leave(&mut self) {
        self.frames.pop();
    }

    pub fn reset(&mut self, name: &str, value: i32) {
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        match frame.iter_mut().find(|counter| counter.0 == name) {
            Some(counter) => counter.1 = value,
            None => frame.push((name.to_string(), value)),
        }
    }

    pub fn set(&mut self, name: &str, value: i32) {
        match self.find_mut(name) {
            Some(counter) => *counter = value,
            None => self.reset(name, value),
        }
    }

    pub fn increment(&mut self, name: &str, by: i32) {
        match self.find_mut(name) {
            Some(counter) => *counter = counter.saturating_add(by),
            None => self.reset(name, by),
        }
    }

    pub fn value(&self, name: &str) -> i32 {
        self.values(name).last().copied().unwrap_or(0)
    }

    pub fn values(&self, name: &str) -> Vec<i32> {
        self.frames
            .iter()
            .filter_map(|frame| frame.iter().find(|counter| counter.0 == name))
            .map(|counter| counter.1)
            .collect()
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut i32> {
        self.frames
            .iter_mut()
            .rev()
            .find_map(|frame| frame.iter_mut().find(|counter| counter.0 == name))
            .map(|counter| &mut counter.1)
    }
}

impl Default for Counters {
    fn default() -> Counters {
        Counters::new()
    }
}
