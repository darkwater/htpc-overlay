use core::ops::{Add, Div, Neg, Sub};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Time(f32);

impl Time {
    pub const ZERO: Self = Time(0.0);

    pub fn seconds(n: impl Into<f64>) -> Self {
        Time(n.into() as f32)
    }

    pub fn minutes(n: impl Into<f64>) -> Self {
        Time(n.into() as f32 * 60.)
    }

    pub fn mmss(self) -> String {
        let minutes = (self.0 / 60.).floor() as u32;
        let seconds = (self.0 % 60.).floor() as u32;
        format!("{}:{:02}", minutes, seconds)
    }
}

impl Add for Time {
    type Output = Time;

    fn add(self, rhs: Self) -> Self::Output {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Time;

    fn sub(self, rhs: Self) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl Neg for Time {
    type Output = Time;

    fn neg(self) -> Self::Output {
        Time(-self.0)
    }
}

impl Div for Time {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Div<f32> for Time {
    type Output = Time;

    fn div(self, rhs: f32) -> Self::Output {
        Time(self.0 / rhs)
    }
}

// pub trait DurationExt {
//     fn as_time(&self) -> Time;
// }

// impl DurationExt for Duration {
//     fn as_time(&self) -> Time {
//         Time(self.as_secs_f32())
//     }
// }
