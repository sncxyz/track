pub mod commands;
mod data;
mod table;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Clone)]
pub enum Position {
    Index(usize),
    Last,
}

#[derive(Clone, Copy)]
pub enum Absolute {
    DateTime(NaiveDateTime),
    Date(NaiveDate),
    Time(NaiveTime),
}

#[derive(Clone, Copy)]
pub enum Bound {
    None,
    Now,
    Absolute(Absolute),
    Ago {
        weeks: u32,
        days: u32,
        hours: u32,
        minutes: u32,
    },
}

impl Bound {
    fn is_none(&self) -> bool {
        if let Bound::None = self {
            true
        } else {
            false
        }
    }
}
