use std::{env, fmt, fs, io, path::PathBuf};

use anyhow::{bail, Result};
use bincode::{deserialize, serialize};
use chrono::{
    serde::{ts_seconds, ts_seconds_option},
    Duration, Local, NaiveDateTime, TimeZone, Utc,
};
use serde::{Deserialize, Serialize};

use super::{Bound, Position};

type DateTime = chrono::DateTime<Utc>;

pub fn create(name: String) -> Result<()> {
    let mut data = Data::read();
    for info in &data.all {
        if info.name == name {
            bail!("error: An activity with this name already exists");
        }
    }
    let mut id = 0;
    loop {
        for info in &data.all {
            if info.id == id {
                id += 1;
                continue;
            }
        }
        break;
    }
    println!("Created new activity \"{}\"", name);
    if data.current.is_none() {
        println!("Set current activity to \"{}\"", name);
        data.current = Some(ActivityInfo::new(name.clone(), id));
    }
    data.all.push(ActivityInfo::new(name, id));
    data.write()?;
    Activity::new().write(id)?;
    Ok(())
}

pub fn set(name: String) -> Result<()> {
    let mut data = Data::read();
    for info in &data.all {
        if info.name == name {
            data.current = Some(info.clone());
            data.write()?;
            println!("Set current activity to \"{name}\"");
            return Ok(());
        }
    }
    bail!("error: No activity with this name exists");
}

pub fn delete(name: String) -> Result<()> {
    let mut data = Data::read();
    let mut i = 0;
    for info in &data.all {
        if info.name == name {
            println!("Are you sure you want to delete activity \"{name}\"? Enter \"y\" if so");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() == "y" {
                let removed = data.all.remove(i);
                if let Some(current) = &data.current {
                    if current.id == removed.id {
                        data.current = None;
                    }
                }
                fs::remove_file(dir().join(removed.id.to_string()))?;
                data.write()?;
                println!("Deleted activity \"{name}\"");
            } else {
                println!("Did not delete activity \"{name}\"");
            }
            return Ok(());
        }
        i += 1;
    }
    bail!("error: No activity with this name exists");
}

pub fn current() -> Result<()> {
    if let Some(info) = &Data::read().current {
        println!("The current activity is \"{}\"", info.name);
    } else {
        println!("There is no activity currently selected");
    }
    Ok(())
}

pub fn all() -> Result<()> {
    let data = Data::read();
    if data.all.is_empty() {
        println!("There are currently no recorded activities");
    } else {
        println!("The recorded activities are:");
        for info in &data.all {
            println!("{}", info.name);
        }
    }
    Ok(())
}

pub fn start() -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    if current.ongoing.is_some() {
        bail!("There is already an ongoing session of activity \"{name}\"");
    }
    current.ongoing = Some(Utc::now());
    let local = local(current.ongoing.unwrap());
    data.write_current(&current)?;
    println!(
        "Started new session of activity \"{name}\" on {} at {}",
        local.format("%d/%m/%y"),
        local.format("%R")
    );
    Ok(())
}

pub fn end(notes: String) -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    if let Some(start) = current.ongoing {
        current.ongoing = None;
        let end = Utc::now();
        current.sessions.push(Session::new(start, end, notes));
        data.write_current(&current)?;
        println!("Ended session of activity \"{name}\"");
        println!("New session:");
        println!("{}", current.get(current.last()));
    } else {
        bail!("error: There is no ongoing session of activity \"{name}\"");
    }
    Ok(())
}

pub fn cancel() -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    if current.ongoing.is_some() {
        current.ongoing = None;
        data.write_current(&current)?;
        println!("Cancelled ongoing session of activity \"{name}\"");
        return Ok(());
    }
    bail!("error: There is no ongoing session of activity \"{name}\"");
}

pub fn ongoing() -> Result<()> {
    let data = Data::read();
    let (current, name) = data.read_current()?;
    if let Some(start) = current.ongoing {
        let local = local(start);
        println!(
            "There is an ongoing session of activity \"{name}\" that started on {} at {}",
            local.format("%d/%m/%y"),
            local.format("%R")
        );
    } else {
        println!("There is no ongoing session of activity \"{name}\"");
    }
    Ok(())
}

pub fn add(start: NaiveDateTime, end: NaiveDateTime, notes: String) -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    let start = parse_date_time(start);
    let end = parse_date_time(end);
    let i = current.add(start, end, notes)?;
    data.write_current(&current)?;
    println!("Added a new session of activity \"{name}\":");
    println!("{}", current.get(i));
    Ok(())
}

pub fn edit(
    pos: Position,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
    notes: Option<String>,
) -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    let i = current.parse_index(pos)?;
    let old_string = current.get(i);
    let old = current.sessions.remove(i);
    let start = start.map(parse_date_time).unwrap_or(old.start);
    let end = end.map(parse_date_time).unwrap_or(old.end);
    let notes = notes.unwrap_or_else(|| old.notes.clone());
    let i = current.add(start, end, notes)?;
    data.write_current(&current)?;
    println!("Edited session of activity \"{name}\" from:");
    println!("{old_string}");
    println!("to:");
    println!("{}", current.get(i));
    Ok(())
}

pub fn remove(pos: Position) -> Result<()> {
    let data = Data::read();
    let (mut current, name) = data.read_current()?;
    let i = current.parse_index(pos)?;
    println!("{}", current.get(i));
    println!();
    println!(
        "Are you sure you want to remove this session from activity \"{name}\"? Enter \"y\" if so"
    );
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim() == "y" {
        current.sessions.remove(i);
        data.write_current(&current)?;
        println!("Removed session");
    } else {
        println!("Did not remove session");
    }
    Ok(())
}

pub fn list(from: Bound, to: Bound) -> Result<()> {
    let data = Data::read();
    let (current, name) = data.read_current()?;
    let (from, to) = current.convert_bounds(from, to)?;
    let (i, j) = current.get_in_range(from, to);
    if i == j {
        println!(
            "There are no sessions from {} in activity \"{name}\"",
            range_display(from, to)
        );
    } else {
        println!(
            "The sessions from {} in activity \"{name}\" are:",
            range_display(from, to)
        );
        for k in i..j {
            println!("{}", current.get(k));
        }
    }
    Ok(())
}

pub fn stats(from: Bound, to: Bound) -> Result<()> {
    let data = Data::read();
    let (current, name) = data.read_current()?;
    let (from, to) = current.convert_bounds(from, to)?;
    let (i, j) = current.get_in_range(from, to);
    if i == j {
        println!(
            "There are no sessions from {} in activity \"{name}\"",
            range_display(from, to)
        );
    } else {
        println!(
            "The session statistics from {} in activity \"{name}\" are:",
            range_display(from, to)
        );
        println!("Number of sessions: {}", j - i);
        let sessions = &current.sessions;
        let mut time = Duration::zero();
        for k in i..j {
            let (mut start, mut end) = (sessions[k].start, sessions[k].end);
            if k == i {
                start = start.max(from);
            }
            if k == j - 1 {
                end = end.min(to);
            }
            time = time + (end - start);
        }
        let total = to - from;
        let proportion = time.num_seconds() as f64 / total.num_seconds() as f64;
        println!("Total time: {}", duration_str(time));
        println!(
            "Average time per day: {}",
            duration_str(Duration::seconds((proportion * 60. * 60. * 24.) as i64))
        );
        println!(
            "Average session length: {}",
            duration_str(time / (j - i) as i32)
        );
        println!(
            "Proportion of time spent on activity: {:.1}%",
            proportion * 100.
        );
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Data {
    current: Option<ActivityInfo>,
    all: Vec<ActivityInfo>,
}

impl Data {
    fn read() -> Data {
        if let Ok(encoded) = fs::read(dir().join("data")) {
            deserialize(&encoded).unwrap()
        } else {
            Self {
                current: None,
                all: Vec::new(),
            }
        }
    }

    fn write(&self) -> Result<()> {
        if !dir().exists() {
            fs::create_dir(dir())?;
        }
        fs::write(dir().join("data"), serialize(self).unwrap())?;
        Ok(())
    }

    fn read_current(&self) -> Result<(Activity, &str)> {
        if let Some(info) = &self.current {
            Ok((
                deserialize(&fs::read(dir().join(info.id.to_string()))?).unwrap(),
                &info.name,
            ))
        } else {
            bail!("error: No activity currently selected")
        }
    }

    fn write_current(&self, activity: &Activity) -> Result<()> {
        activity.write(self.current.as_ref().unwrap().id)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ActivityInfo {
    name: String,
    id: u32,
}

impl ActivityInfo {
    fn new(name: String, id: u32) -> Self {
        Self { name, id }
    }
}

#[derive(Serialize, Deserialize)]
struct Activity {
    #[serde(with = "ts_seconds_option")]
    ongoing: Option<DateTime>,
    sessions: Vec<Session>,
}

impl Activity {
    fn new() -> Self {
        Self {
            ongoing: None,
            sessions: Vec::new(),
        }
    }

    fn write(&self, id: u32) -> Result<()> {
        fs::write(dir().join(id.to_string()), serialize(self).unwrap())?;
        Ok(())
    }

    fn add(&mut self, start: DateTime, end: DateTime, notes: String) -> Result<usize> {
        if end <= start {
            bail!("error: Session must end after it starts");
        }
        if end > Utc::now() {
            bail!("error: Session cannot have ended in the future");
        }
        let mut i = 0;
        while i < self.sessions.len() {
            let other = &self.sessions[i];
            if end >= other.start {
                if other.end >= start {
                    bail!("error: Session overlaps existing session");
                }
            } else {
                break;
            }
            i += 1;
        }
        self.sessions.insert(i, Session::new(start, end, notes));
        Ok(i)
    }

    fn get(&self, index: usize) -> String {
        format!("{:3}. {}", index, self.sessions[index])
    }

    fn last(&self) -> usize {
        if self.sessions.is_empty() {
            return 0;
        }
        self.sessions.len() - 1
    }

    fn parse_index(&self, pos: Position) -> Result<usize> {
        if self.sessions.is_empty() {
            bail!("error: There are no recorded sessions of the current activity");
        }
        let i = match pos {
            Position::Last => self.last(),
            Position::Index(i) => i,
        };
        if i >= self.sessions.len() {
            bail!("error: No session of the current activity with this index exists")
        }
        Ok(i)
    }

    fn convert_bounds(&self, from: Bound, to: Bound) -> Result<(DateTime, DateTime)> {
        if self.sessions.is_empty() {
            bail!("There are no recorded sessions of the current activity");
        }
        let now = Utc::now();
        let from = match from {
            Bound::DateTime(naive) => parse_date_time(naive),
            Bound::Date(date) => parse_date_time(date.and_hms(0, 0, 0)),
            Bound::Ago {
                weeks,
                days,
                hours,
                minutes,
            } => {
                now - Duration::minutes(
                    minutes as i64
                        + hours as i64 * 60
                        + days as i64 * 24 * 60
                        + weeks as i64 * 7 * 24 * 60,
                )
            }
            Bound::None => self.sessions[0].start,
            _ => unreachable!(),
        };
        let to = match to {
            Bound::DateTime(naive) => parse_date_time(naive),
            Bound::Date(date) => {
                if Local.from_local_date(&date).unwrap() == Local::now().date() {
                    now
                } else {
                    parse_date_time(date.succ().and_hms(0, 0, 0))
                }
            }
            Bound::None => self.sessions[self.last()].end,
            Bound::Now => now,
            _ => unreachable!(),
        };
        if from >= to {
            bail!("error: Start of range must be before end");
        }
        if to > now {
            bail!("error: Range must not end in the future");
        }
        Ok((from, to))
    }

    fn get_in_range(&self, from: DateTime, to: DateTime) -> (usize, usize) {
        let mut i = 0;
        let len = self.sessions.len();
        while i < len && self.sessions[i].end <= from {
            i += 1;
        }
        let mut j = i;
        while j < len && self.sessions[j].start < to {
            j += 1;
        }
        (i, j)
    }
}

#[derive(Serialize, Deserialize)]
struct Session {
    #[serde(with = "ts_seconds")]
    start: DateTime,
    #[serde(with = "ts_seconds")]
    end: DateTime,
    notes: String,
}

impl Session {
    fn new(start: DateTime, end: DateTime, notes: String) -> Self {
        Self { start, end, notes }
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let duration = duration_str(self.end - self.start);
        write!(f, "{} ({duration})", range_display(self.start, self.end))?;
        if !self.notes.is_empty() {
            write!(f, " - {}", self.notes)?;
        }
        Ok(())
    }
}

fn parse_date_time(naive: NaiveDateTime) -> DateTime {
    let local = Local.from_local_datetime(&naive).unwrap();
    local.into()
}

fn duration_str(duration: Duration) -> String {
    let hours = duration.num_hours();
    let mins = duration.num_minutes() - hours * 60;
    format!("{hours}h {mins}m")
}

fn range_display(from: DateTime, to: DateTime) -> String {
    let (from, to) = (local(from), local(to));
    let to_format = if from.date() == to.date() {
        "%R"
    } else {
        "%d/%m/%y %R"
    };
    format!("{} to {}", from.format("%d/%m/%y %R"), to.format(to_format))
}

fn dir() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.push("data");
    path
}

fn local(date_time: DateTime) -> chrono::DateTime<Local> {
    date_time.into()
}