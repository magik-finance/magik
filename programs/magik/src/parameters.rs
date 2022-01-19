pub struct Parameters {}

impl Parameters {
    pub const MAX_PERCENT: u64 = 50;

    pub fn verify_percent(percent: u64) {
        assert_eq!(percent <= Parameters::MAX_PERCENT, true);
    }
}
