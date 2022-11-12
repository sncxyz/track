use std::{
    collections::HashSet,
    fmt, fs,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};
use bincode::{deserialize, serialize};
use chrono::{
    serde::{ts_seconds, ts_seconds_option},
    Duration, Local, NaiveDateTime, TimeZone, Utc,
};
use serde::{Deserialize, Serialize};

use super::{Absolute, Bound, Position};

type DateTime = chrono::DateTime<Utc>;

pub fn create(name: String) -> Result<()> {
    let mut data = Data::read()?;
    for info in &data.all {
        if info.name == name {
            bail!("error: An activity with this name already exists");
        }
    }
    let taken: HashSet<_> = data.all.iter().map(|info| info.id).collect();
    let mut id = 0;
    while taken.contains(&id) {
        id += 1;
    }
    data.all.push(ActivityInfo::new(name.clone(), id));
    data.current = Some(ActivityInfo::new(name.clone(), id));
    data.write()?;
    Activity::new().write(id)?;
    println!("Created new activity \"{}\"", name);
    println!("Set current activity to \"{}\"", name);
    Ok(())
}

pub fn set(name: String) -> Result<()> {
    let mut data = Data::read()?;
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

pub fn rename(from: String, to: String) -> Result<()> {
    let mut data = Data::read()?;
    for info in &mut data.all {
        if info.name == from {
            info.name = to.clone();
            if let Some(current) = &data.current {
                if current.id == info.id {
                    data.current = Some(info.clone());
                }
            }
            data.write()?;
            println!("Renamed activity \"{from}\" to \"{to}\"");
            return Ok(());
        }
    }
    bail!("error: No activity with this name exists")
}

pub fn delete(name: String) -> Result<()> {
    let mut data = Data::read()?;
    for (i, info) in data.all.iter().enumerate() {
        if info.name == name {
            print!("Are you sure you want to delete activity \"{name}\"? Enter \"y\" if so: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() == "y" {
                let removed = data.all.remove(i);
                if let Some(current) = &data.current {
                    if current.id == removed.id {
                        data.current = None;
                    }
                }
                fs::remove_file(dir()?.join(removed.id.to_string()))?;
                data.write()?;
                println!("Deleted activity \"{name}\"");
            } else {
                println!("Did not delete activity \"{name}\"");
            }
            return Ok(());
        }
    }
    bail!("error: No activity with this name exists");
}

pub fn current() -> Result<()> {
    if let Some(info) = &Data::read()?.current {
        println!("The current activity is \"{}\"", info.name);
    } else {
        println!("There is no activity currently selected");
    }
    Ok(())
}

pub fn all() -> Result<()> {
    let data = Data::read()?;
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
    let data = Data::read()?;
    let (mut current, name) = data.read_current()?;
    if current.ongoing.is_some() {
        bail!("There is already an ongoing session of activity \"{name}\"");
    }
    current.ongoing = Some(Utc::now());
    let local = to_local(current.ongoing.unwrap());
    data.write_current(&current)?;
    println!(
        "Started new session of activity \"{name}\" on {} at {}",
        local.format("%d/%m/%y"),
        local.format("%R")
    );
    Ok(())
}

pub fn end(notes: String) -> Result<()> {
    let data = Data::read()?;
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
    let data = Data::read()?;
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
    let data = Data::read()?;
    let (current, name) = data.read_current()?;
    if let Some(start) = current.ongoing {
        let local = to_local(start);
        println!(
            "There is an ongoing session of activity \"{name}\" that started on {} at {}",
            local.format("%d/%m/%y"),
            local.format("%R")
        );
        println!("Current duration: {}", dur_to_string(Utc::now() - start));
    } else {
        println!("There is no ongoing session of activity \"{name}\"");
    }
    Ok(())
}

pub fn add(start: Absolute, end: Absolute, notes: String) -> Result<()> {
    let data = Data::read()?;
    let (mut current, name) = data.read_current()?;
    let start = parse_start(start);
    let end = parse_end(end, start);
    let i = current.add(start, end, notes)?;
    data.write_current(&current)?;
    println!("Added a new session of activity \"{name}\":");
    println!("{}", current.get(i));
    Ok(())
}

pub fn edit(
    pos: Position,
    start: Option<Absolute>,
    end: Option<Absolute>,
    notes: Option<String>,
) -> Result<()> {
    let data = Data::read()?;
    let (mut current, name) = data.read_current()?;
    let i = current.parse_index(pos)?;
    if start.is_none() && end.is_none() && notes.is_none() {
        bail!("error: No edits specified")
    }
    let old_string = current.get(i);
    let old = current.sessions.remove(i);
    let start = start.map(parse_start).unwrap_or(old.start);
    let end = end.map(|abs| parse_end(abs, start)).unwrap_or(old.end);
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
    let data = Data::read()?;
    let (mut current, name) = data.read_current()?;
    let i = current.parse_index(pos)?;
    println!("{}", current.get(i));
    print!("Are you sure you want to remove this session from activity \"{name}\"? Enter \"y\" if so: ");
    io::stdout().flush()?;
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
    let all = from.is_none() && to.is_none();
    let data = Data::read()?;
    let (current, name) = data.read_current()?;
    let (from, to) = current.convert_bounds(from, to)?;
    let (i, j) = current.get_in_range(from, to);
    let text = format!(
        "{}in activity \"{name}\"",
        if all {
            String::new()
        } else {
            let range = range_to_string(from, to);
            format!("from {} ", range)
        }
    );
    if i == j {
        println!("There are no sessions {text}");
    } else {
        println!("The sessions {text} are:");
        for k in i..j {
            println!("{}", current.get(k));
        }
    }
    Ok(())
}

pub fn stats(from: Bound, to: Bound) -> Result<()> {
    let all = from.is_none() && to.is_none();
    let data = Data::read()?;
    let (current, name) = data.read_current()?;
    let (from, to) = current.convert_bounds(from, to)?;
    let (i, j) = current.get_in_range(from, to);
    let text = format!(
        "{}in activity \"{name}\"",
        if all {
            String::new()
        } else {
            let range = range_to_string(from, to);
            let duration = dur_stat(to - from);
            format!("from {} ({}) ", range, duration)
        }
    );
    if i == j {
        println!("There are no sessions {text}");
    } else {
        println!("The sessions statistics {text} are:");
        println!("Number of sessions: {}", j - i);
        let mut time = Duration::zero();
        for (k, session) in current.sessions.iter().enumerate().take(j).skip(i) {
            let (mut start, mut end) = (session.start, session.end);
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
        println!("Total time: {}", dur_stat(time));
        println!(
            "Average time per day: {}",
            dur_stat(Duration::seconds((proportion * 60. * 60. * 24.) as i64))
        );
        println!(
            "Average session length: {}",
            dur_stat(time / (j - i) as i32)
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
    fn read() -> Result<Data> {
        Ok(if let Ok(encoded) = fs::read(dir()?.join("data")) {
            deserialize(&encoded)?
        } else {
            Self {
                current: None,
                all: Vec::new(),
            }
        })
    }

    fn write(&self) -> Result<()> {
        if !dir()?.exists() {
            fs::create_dir(dir()?)?;
        }
        fs::write(dir()?.join("data"), serialize(self)?)?;
        Ok(())
    }

    fn read_current(&self) -> Result<(Activity, &str)> {
        if let Some(info) = &self.current {
            Ok((
                deserialize(&fs::read(dir()?.join(info.id.to_string()))?)?,
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
        fs::write(dir()?.join(id.to_string()), serialize(self)?)?;
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
                    bail!("error: Session overlaps existing session:\n{}", self.get(i));
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
        format!("{:3}. {}", index + 1, self.sessions[index])
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
            Position::Index(i) => i - 1,
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
            Bound::Absolute(abs) => parse_start(abs),
            Bound::Ago {
                weeks,
                days,
                hours,
                minutes,
            } => {
                if weeks == 0 && days == 0 && hours == 0 && minutes == 0 {
                    self.sessions[0].start
                } else {
                    now - Duration::minutes(
                        minutes as i64
                            + hours as i64 * 60
                            + days as i64 * 24 * 60
                            + weeks as i64 * 7 * 24 * 60,
                    )
                }
            }
            Bound::None => self.sessions[0].start,
            _ => unreachable!(),
        };
        let to = match to {
            Bound::Absolute(abs) => parse_end(abs, from),
            Bound::None => self.sessions[self.last()].end,
            Bound::Now => now,
            _ => unreachable!(),
        };
        if from >= to {
            bail!("error: Start of range must be before end");
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
        let range = range_to_string(self.start, self.end);
        let duration = dur_to_string(self.end - self.start);
        write!(f, "{} ({})", range, duration)?;
        if !self.notes.is_empty() {
            write!(f, " - {}", self.notes)?;
        }
        Ok(())
    }
}

fn dur_to_string(duration: Duration) -> String {
    let mut hours = duration.num_hours();
    let mut mins = duration.num_minutes() - hours * 60;
    let secs = duration.num_seconds() - hours * 60 * 60 - mins * 60;
    if secs > 0 {
        mins += 1;
    }
    if mins == 60 {
        hours += 1;
        mins = 0;
    }
    format!("{hours}h {mins}m")
}

fn dur_stat(duration: Duration) -> String {
    let hours = duration.num_hours();
    let mins = duration.num_minutes() - hours * 60;
    let secs = duration.num_seconds() - hours * 60 * 60 - mins * 60;
    if hours == 0 && mins == 0 {
        format!("{secs}s")
    } else {
        let hm = if hours == 0 {
            format!("{mins}m")
        } else if mins == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {mins}m")
        };
        if secs == 0 {
            hm
        } else {
            format!("{hm} {secs}s")
        }
    }
}

fn range_to_string(from: DateTime, to: DateTime) -> String {
    let (from, to) = (to_local(from), to_local(to));
    let to_format = if from.date_naive() == to.date_naive() {
        "%R"
    } else {
        "%d/%m/%y %R"
    };
    format!("{} to {}", from.format("%d/%m/%y %R"), to.format(to_format),)
}

fn to_local(date_time: DateTime) -> chrono::DateTime<Local> {
    date_time.into()
}

fn parse_dt(naive: NaiveDateTime) -> DateTime {
    Local.from_local_datetime(&naive).unwrap().into()
}

fn parse_start(abs: Absolute) -> DateTime {
    parse_dt(match abs {
        Absolute::DateTime(naive) => naive,
        Absolute::Date(naive) => naive.and_hms_opt(0, 0, 0).unwrap(),
        Absolute::Time(naive) => Local::now().date_naive().and_time(naive),
    })
}

fn parse_end(abs: Absolute, start: DateTime) -> DateTime {
    parse_dt(match abs {
        Absolute::DateTime(naive) => naive,
        Absolute::Date(naive) => naive.succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap(),
        Absolute::Time(naive) => to_local(start).date_naive().and_time(naive),
    })
}

fn dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .ok_or_else(|| anyhow!("error: Failed to find user data directory"))?
        .join("track"))
}
