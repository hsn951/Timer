use crate::store::Settings;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Stopwatch,
    Countdown,
    Pomodoro,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Work,
    Short,
    Long,
}

#[derive(Serialize, Clone, Debug)]
pub struct Snapshot {
    pub kind: Kind,
    pub phase: Phase,
    pub round: u32,
    pub rounds: u32,
    pub label: String,
    pub preset_id: Option<String>,
    pub target_ms: u64,
    pub elapsed_ms: u64,
    pub remaining_ms: u64,
    pub running: bool,
    pub finished: bool,
    pub laps: Vec<u64>,
}

pub struct Engine {
    pub kind: Kind,
    pub phase: Phase,
    pub round: u32,
    pub label: String,
    pub preset_id: Option<String>,
    pub target_ms: u64,
    pub accumulated_ms: u64,
    pub started: Option<Instant>,
    pub finished: bool,
    pub laps: Vec<u64>,
}

fn phase_ms(phase: Phase, settings: &Settings) -> u64 {
    let minutes = match phase {
        Phase::Work => settings.work_min.max(1),
        Phase::Short => settings.break_min.max(1),
        Phase::Long => settings.long_break_min.max(1),
    };
    minutes as u64 * 60_000
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Work => "专注",
        Phase::Short => "小憩",
        Phase::Long => "长休息",
    }
}

impl Engine {
    pub fn new(_settings: &Settings) -> Self {
        Self {
            kind: Kind::Countdown,
            phase: Phase::Work,
            round: 1,
            label: "倒计时".to_string(),
            preset_id: None,
            target_ms: 5 * 60_000,
            accumulated_ms: 0,
            started: None,
            finished: false,
            laps: Vec::new(),
        }
    }

    pub fn running(&self) -> bool {
        self.started.is_some()
    }

    pub fn elapsed_ms(&self, now: Instant) -> u64 {
        self.accumulated_ms
            + self
                .started
                .map(|s| now.saturating_duration_since(s).as_millis() as u64)
                .unwrap_or(0)
    }

    pub fn remaining_ms(&self, now: Instant) -> u64 {
        match self.kind {
            Kind::Stopwatch => 0,
            _ => self.target_ms.saturating_sub(self.elapsed_ms(now)),
        }
    }

    pub fn snapshot(&self, now: Instant, rounds: u32) -> Snapshot {
        Snapshot {
            kind: self.kind,
            phase: self.phase,
            round: self.round,
            rounds,
            label: self.label.clone(),
            preset_id: self.preset_id.clone(),
            target_ms: self.target_ms,
            elapsed_ms: self.elapsed_ms(now),
            remaining_ms: self.remaining_ms(now),
            running: self.running(),
            finished: self.finished,
            laps: self.laps.clone(),
        }
    }

    pub fn start(&mut self, settings: &Settings) {
        if self.running() {
            return;
        }
        if self.finished {
            self.finished = false;
            self.accumulated_ms = 0;
        }
        if self.kind == Kind::Pomodoro && self.target_ms == 0 {
            self.target_ms = phase_ms(self.phase, settings);
        }
        if self.kind == Kind::Countdown && self.target_ms == 0 {
            self.target_ms = 5 * 60_000;
        }
        if self.kind != Kind::Stopwatch && self.accumulated_ms >= self.target_ms {
            self.accumulated_ms = 0;
        }
        self.started = Some(Instant::now());
    }

    pub fn pause(&mut self, now: Instant) {
        if let Some(started) = self.started.take() {
            self.accumulated_ms += now.saturating_duration_since(started).as_millis() as u64;
        }
    }

    pub fn reset(&mut self, settings: &Settings) {
        self.started = None;
        self.accumulated_ms = 0;
        self.finished = false;
        self.laps.clear();
        self.preset_id = None;
        if self.kind == Kind::Pomodoro {
            self.phase = Phase::Work;
            self.round = 1;
            self.target_ms = phase_ms(Phase::Work, settings);
            self.label = phase_name(Phase::Work).to_string();
        }
    }

    pub fn lap(&mut self, now: Instant) {
        if self.kind == Kind::Stopwatch && self.running() {
            self.laps.insert(0, self.elapsed_ms(now));
        }
    }

    pub fn switch_kind(&mut self, kind: Kind, settings: &Settings) {
        self.kind = kind;
        self.started = None;
        self.accumulated_ms = 0;
        self.finished = false;
        self.laps.clear();
        self.preset_id = None;
        match kind {
            Kind::Stopwatch => {
                self.target_ms = 0;
                self.label = "秒表".to_string();
            }
            Kind::Countdown => {
                if self.target_ms == 0 || self.target_ms > 24 * 3600 * 1000 {
                    self.target_ms = 5 * 60_000;
                }
                self.label = "倒计时".to_string();
            }
            Kind::Pomodoro => {
                self.phase = Phase::Work;
                self.round = 1;
                self.target_ms = phase_ms(Phase::Work, settings);
                self.label = phase_name(Phase::Work).to_string();
            }
        }
    }

    pub fn set_target(&mut self, ms: u64) {
        if self.kind == Kind::Stopwatch {
            return;
        }
        self.target_ms = ms.max(1000);
        if !self.running() {
            self.accumulated_ms = 0;
            self.finished = false;
        }
    }

    pub fn add_time(&mut self, ms: u64) {
        if self.kind == Kind::Stopwatch {
            return;
        }
        if self.finished {
            self.finished = false;
            self.accumulated_ms = 0;
            self.target_ms = ms.max(1000);
        } else {
            self.target_ms += ms;
        }
    }

    pub fn skip_phase(&mut self, settings: &Settings) {
        match self.kind {
            Kind::Pomodoro => {
                self.advance(settings);
                self.accumulated_ms = 0;
                self.finished = false;
                self.started = Some(Instant::now());
            }
            Kind::Countdown => {
                self.accumulated_ms = self.target_ms;
                self.started = None;
                self.finished = true;
            }
            Kind::Stopwatch => {}
        }
    }

    pub fn advance(&mut self, settings: &Settings) {
        match self.phase {
            Phase::Work => {
                let rounds = settings.rounds.max(1);
                self.phase = if self.round % rounds == 0 {
                    Phase::Long
                } else {
                    Phase::Short
                };
            }
            _ => {
                self.round = self.round.saturating_add(1);
                self.phase = Phase::Work;
            }
        }
        self.target_ms = phase_ms(self.phase, settings);
        self.label = phase_name(self.phase).to_string();
    }

    pub fn finish(&mut self) {
        self.started = None;
        self.accumulated_ms = self.target_ms;
        self.finished = true;
    }

    /// 从休眠/待机恢复时，把单调时钟漏掉的时长补回来。
    pub fn compensate(&mut self, extra_ms: u64, now: Instant) {
        if self.running() {
            self.accumulated_ms = self.accumulated_ms.saturating_add(extra_ms);
            self.started = Some(now);
        }
    }
}
