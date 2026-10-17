//! Time-of-day queue scheduling and "when everything is done" detection.
//!
//! Pure functions so the timing rules can be tested without a clock; the
//! desktop app calls them from a timer.

use chrono::{Datelike, Local, Timelike};

use crate::settings::Schedule;
use crate::task::{TaskSnapshot, TaskState};

const DAY: u32 = 24 * 60;
const WEEK: u32 = 7 * DAY;

/// A local wall-clock moment, at minute resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalStamp {
    /// 0 = Monday.
    pub weekday: u32,
    pub minute: u32,
}

impl LocalStamp {
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            weekday: now.weekday().num_days_from_monday(),
            minute: now.hour() * 60 + now.minute(),
        }
    }

    fn week_minute(self) -> u32 {
        self.weekday.min(6) * DAY + self.minute.min(DAY - 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleAction {
    Start,
    Stop,
}

/// `HH:MM` with a 24-hour clock.
pub fn is_time(s: &str) -> bool {
    parse_time(s).is_some()
}

pub fn parse_time(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 {
        return None;
    }
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// What the schedule asks for between two timer ticks, `(prev, now]`.
///
/// Comparing ranges rather than "is it 02:00 now" means a tick that lands a
/// little late, or a computer that slept through the start time and woke up
/// later, still acts. If both times passed, the later one wins.
pub fn due(schedule: &Schedule, prev: LocalStamp, now: LocalStamp) -> Option<ScheduleAction> {
    if !schedule.enabled {
        return None;
    }
    let start = parse_time(&schedule.start);
    let stop = parse_time(&schedule.stop);
    let p = prev.week_minute();
    let mut n = now.week_minute();
    if n < p {
        n += WEEK;
    }
    if n == p {
        return None;
    }

    let mut latest: Option<(u32, ScheduleAction)> = None;
    for (time, action) in [(start, ScheduleAction::Start), (stop, ScheduleAction::Stop)] {
        let Some(time) = time else { continue };
        for (day, enabled) in schedule.days.iter().enumerate() {
            if !enabled {
                continue;
            }
            let at = day as u32 * DAY + time;
            for candidate in [at, at + WEEK] {
                if p < candidate && candidate <= n && latest.is_none_or(|(t, _)| candidate > t) {
                    latest = Some((candidate, action));
                }
            }
        }
    }
    latest.map(|(_, action)| action)
}

/// Downloads that still need to run: anything running or waiting in the queue.
pub fn active_count(tasks: &[TaskSnapshot]) -> usize {
    tasks
        .iter()
        .filter(|t| t.record.state.is_running() || t.record.state == TaskState::Queued)
        .count()
}

/// Tracks the "when all downloads finish" trigger. It fires once when the
/// number of active downloads drops to zero after having been above zero,
/// so arming it with an empty queue waits for the next batch instead of
/// firing at once.
#[derive(Debug, Default)]
pub struct DoneWatcher {
    saw_work: bool,
}

impl DoneWatcher {
    pub fn arm(&mut self, active_now: usize) {
        self.saw_work = active_now > 0;
    }

    /// Feed the current number of active downloads; true exactly once when
    /// they are all finished.
    pub fn observe(&mut self, active_now: usize) -> bool {
        if active_now > 0 {
            self.saw_work = true;
            false
        } else if self.saw_work {
            self.saw_work = false;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(weekday: u32, hhmm: &str) -> LocalStamp {
        LocalStamp {
            weekday,
            minute: parse_time(hhmm).unwrap(),
        }
    }

    fn schedule(start: &str, stop: &str) -> Schedule {
        Schedule {
            enabled: true,
            start: start.into(),
            stop: stop.into(),
            days: [true; 7],
        }
    }

    #[test]
    fn parses_times() {
        assert_eq!(parse_time("02:30"), Some(150));
        assert_eq!(parse_time("2:05"), Some(125));
        assert_eq!(parse_time("23:59"), Some(1439));
        for bad in ["24:00", "12:60", "1230", "", "ab:cd", "1:5", "001:00"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn fires_when_a_time_passes_between_ticks() {
        let s = schedule("02:00", "07:00");
        assert_eq!(
            due(&s, at(0, "01:59"), at(0, "02:00")),
            Some(ScheduleAction::Start)
        );
        assert_eq!(due(&s, at(0, "02:00"), at(0, "02:01")), None, "only once");
        assert_eq!(
            due(&s, at(0, "06:59"), at(0, "07:00")),
            Some(ScheduleAction::Stop)
        );
        assert_eq!(due(&s, at(0, "03:00"), at(0, "03:00")), None);
    }

    #[test]
    fn late_ticks_and_sleep_still_act() {
        let s = schedule("02:00", "07:00");
        // Woke up at 03:10 after sleeping since 01:00.
        assert_eq!(
            due(&s, at(2, "01:00"), at(2, "03:10")),
            Some(ScheduleAction::Start)
        );
        // Slept through both: the later one, stop, is what applies now.
        assert_eq!(
            due(&s, at(2, "01:00"), at(2, "08:00")),
            Some(ScheduleAction::Stop)
        );
    }

    #[test]
    fn handles_midnight_and_week_wrap() {
        let s = schedule("00:00", "");
        assert_eq!(
            due(&s, at(6, "23:59"), at(0, "00:00")),
            Some(ScheduleAction::Start)
        );
        let late = schedule("23:30", "00:30");
        assert_eq!(
            due(&late, at(3, "23:00"), at(3, "23:45")),
            Some(ScheduleAction::Start)
        );
        assert_eq!(
            due(&late, at(3, "23:45"), at(4, "00:31")),
            Some(ScheduleAction::Stop)
        );
    }

    #[test]
    fn respects_days_and_switch() {
        let mut s = schedule("02:00", "");
        s.days = [false, true, false, false, false, false, false];
        assert_eq!(
            due(&s, at(0, "01:00"), at(0, "03:00")),
            None,
            "Monday is off"
        );
        assert_eq!(
            due(&s, at(1, "01:00"), at(1, "03:00")),
            Some(ScheduleAction::Start)
        );
        s.enabled = false;
        assert_eq!(due(&s, at(1, "01:00"), at(1, "03:00")), None);
    }

    #[test]
    fn done_watcher_fires_once_after_work() {
        let mut w = DoneWatcher::default();
        w.arm(0);
        assert!(!w.observe(0), "armed on an empty queue: wait for work");
        assert!(!w.observe(2));
        assert!(!w.observe(1));
        assert!(w.observe(0));
        assert!(!w.observe(0), "fires once");

        let mut w = DoneWatcher::default();
        w.arm(3);
        assert!(w.observe(0));
    }
}
