use core::time::Duration;

#[derive(Debug, Clone, Copy, Default)]
pub enum SeekSpeed {
    Second,
    #[default]
    FiveSeconds,
    ThirtySeconds,
    Minute,
    TenMinutes,
}

impl SeekSpeed {
    pub fn duration(self) -> Duration {
        match self {
            SeekSpeed::Second => Duration::from_secs(1),
            SeekSpeed::FiveSeconds => Duration::from_secs(5),
            SeekSpeed::ThirtySeconds => Duration::from_secs(30),
            SeekSpeed::Minute => Duration::from_secs(60),
            SeekSpeed::TenMinutes => Duration::from_secs(600),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SeekSpeed::Second => "1s",
            SeekSpeed::FiveSeconds => "5s",
            SeekSpeed::ThirtySeconds => "30s",
            SeekSpeed::Minute => "1m",
            SeekSpeed::TenMinutes => "10m",
        }
    }

    pub fn longer(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => Some(SeekSpeed::FiveSeconds),
            SeekSpeed::FiveSeconds => Some(SeekSpeed::ThirtySeconds),
            SeekSpeed::ThirtySeconds => Some(SeekSpeed::Minute),
            SeekSpeed::Minute => Some(SeekSpeed::TenMinutes),
            SeekSpeed::TenMinutes => None,
        }
    }

    pub fn shorter(self) -> Option<Self> {
        match self {
            SeekSpeed::Second => None,
            SeekSpeed::FiveSeconds => Some(SeekSpeed::Second),
            SeekSpeed::ThirtySeconds => Some(SeekSpeed::FiveSeconds),
            SeekSpeed::Minute => Some(SeekSpeed::ThirtySeconds),
            SeekSpeed::TenMinutes => Some(SeekSpeed::Minute),
        }
    }
}
