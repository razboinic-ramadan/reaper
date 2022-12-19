use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

#[derive(Clone)]
pub struct Duration {
    pub years: u64,
    pub months: u64,
    pub weeks: u64,
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: u64,
    pub string: String
}

impl Duration {
    pub fn new(duration_string: String) -> Duration {
        let mut duration = Duration {
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
            string: duration_string.clone()
        };

        let reg: Regex = Regex::new(r"(\d+)\S*(y|mo|w|d|h|m|s)").unwrap();
        for capture in reg.captures_iter(&duration_string.to_lowercase()) {
            let value = capture.get(1).unwrap().as_str().parse::<u64>().unwrap();
            let unit = capture.get(2).unwrap().as_str();

            match unit {
                "y" => duration.years = value,
                "mo" => duration.months = value,
                "w" => duration.weeks = value,
                "d" => duration.days = value,
                "h" => duration.hours = value,
                "m" => duration.minutes = value,
                "s" => duration.seconds = value,
                _ => {}
            }
        }

        duration
    }

    pub fn to_unix_timestamp(&self) -> u64 {
        if self.is_permanent() {
            return 0;
        }
        let mut timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        timestamp += self.seconds;
        timestamp += self.minutes * 60;
        timestamp += self.hours * 60 * 60;
        timestamp += self.days * 60 * 60 * 24;
        timestamp += self.weeks * 60 * 60 * 24 * 7;
        timestamp += self.months * 60 * 60 * 24 * 30;
        timestamp += self.years * 60 * 60 * 24 * 365;

        timestamp
    }

    pub fn is_permanent(&self) -> bool {
        self.years == 0 && self.months == 0 && self.weeks == 0 && self.days == 0 && self.hours == 0 && self.minutes == 0 && self.seconds == 0
    }
}