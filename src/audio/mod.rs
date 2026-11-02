pub mod policy_config;
pub mod wasapi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub channels: u16,
}

impl AudioFormat {
    pub fn new(sample_rate: u32, bit_depth: u16) -> Self {
        Self {
            sample_rate,
            bit_depth,
            channels: 2,
        }
    }

    pub fn display_str(&self) -> String {
        let khz = if self.sample_rate.is_multiple_of(1000) {
            format!("{} kHz", self.sample_rate / 1000)
        } else {
            format!("{:.1} kHz", self.sample_rate as f64 / 1000.0)
        };
        format!("{} / {}-bit", khz, self.bit_depth)
    }
}
