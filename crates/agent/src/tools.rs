//! Productivity widgets whose state lives on the PC: the focus timer, the
//! to-do list and the process list. Keeping the state here means it survives
//! the phone reconnecting and is the same on every connected phone.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::time::sleep;

use crate::protocol::Action;
use crate::App;

pub const TIMER: &str = "timer.state";
pub const TODOS: &str = "todo.list";
pub const PROCESSES: &str = "system.processes";

const MAX_TODOS: usize = 100;
const MAX_TODO_LEN: usize = 200;

/// Killing these takes Windows (or the agent) down with them.
const PROTECTED: &[&str] = &[
    "system", "registry", "idle", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe",
    "svchost.exe", "dwm.exe", "fontdrvhost.exe", "memory compression", "secure system", "system idle process",
];

pub struct Tools {
    timer: Mutex<Timer>,
    todos: Mutex<Vec<Todo>>,
    todo_path: PathBuf,
    /// The processes behind each name in the list last sent to the phone.
    /// Only these can be ended, so the phone cannot name an arbitrary process.
    listed: Mutex<HashMap<String, Vec<Pid>>>,
}

// ---- focus timer ------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Focus,
    Break,
}

struct Timer {
    phase: Phase,
    focus: Duration,
    rest: Duration,
    /// When the running phase ends; `None` while paused.
    deadline: Option<Instant>,
    /// Time left in the phase while paused.
    left: Duration,
    /// Finished focus rounds.
    rounds: u32,
    /// Goes up each time a phase runs out, so the phone knows when to buzz.
    finished: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimerState {
    phase: &'static str,
    running: bool,
    remaining_ms: u64,
    total_ms: u64,
    rounds: u32,
    finished: u32,
}

impl Timer {
    fn new() -> Self {
        let focus = Duration::from_secs(25 * 60);
        Self { phase: Phase::Focus, focus, rest: Duration::from_secs(5 * 60), deadline: None, left: focus, rounds: 0, finished: 0 }
    }

    fn length(&self) -> Duration {
        if self.phase == Phase::Focus { self.focus } else { self.rest }
    }

    fn remaining(&self, now: Instant) -> Duration {
        self.deadline.map_or(self.left, |end| end.saturating_duration_since(now))
    }

    /// Moves on when the running phase has run out: focus rolls straight into
    /// the break, the break ends paused and waits for the next start.
    fn tick(&mut self, now: Instant) {
        if !self.deadline.is_some_and(|end| now >= end) {
            return;
        }
        self.finished += 1;
        if self.phase == Phase::Focus {
            self.rounds += 1;
            self.phase = Phase::Break;
            self.deadline = Some(now + self.rest);
        } else {
            self.phase = Phase::Focus;
            self.deadline = None;
            self.left = self.focus;
        }
    }

    fn toggle(&mut self, now: Instant, focus: Option<u32>, rest: Option<u32>) {
        let minutes = |m: u32| Duration::from_secs(u64::from(m.clamp(1, 180)) * 60);
        match self.deadline {
            Some(_) => {
                self.left = self.remaining(now);
                self.deadline = None;
            }
            None => {
                // New lengths apply to a phase that has not started yet.
                let untouched = self.left == self.length();
                if let Some(m) = focus {
                    self.focus = minutes(m);
                }
                if let Some(m) = rest {
                    self.rest = minutes(m);
                }
                if untouched {
                    self.left = self.length();
                }
                self.deadline = Some(now + self.left);
            }
        }
    }

    fn reset(&mut self) {
        self.phase = Phase::Focus;
        self.deadline = None;
        self.left = self.focus;
        self.rounds = 0;
    }

    fn skip(&mut self, now: Instant) {
        let running = self.deadline.is_some();
        self.phase = if self.phase == Phase::Focus { Phase::Break } else { Phase::Focus };
        self.left = self.length();
        self.deadline = running.then(|| now + self.left);
    }

    fn state(&self, now: Instant) -> TimerState {
        TimerState {
            phase: if self.phase == Phase::Focus { "focus" } else { "break" },
            running: self.deadline.is_some(),
            // Whole seconds, so the phone is not sent a new value 10× a second.
            remaining_ms: (self.remaining(now).as_millis() as u64).div_ceil(1000) * 1000,
            total_ms: self.length().as_millis() as u64,
            rounds: self.rounds,
            finished: self.finished,
        }
    }
}

// ---- to-do list -------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    text: String,
    #[serde(default)]
    done: bool,
}

// ---- processes --------------------------------------------------------------

#[derive(Serialize)]
struct ProcessRow {
    name: String,
    /// Share of the whole CPU, 0–100.
    cpu: f32,
    /// Bytes of memory.
    mem: u64,
    /// How many processes share the name.
    count: usize,
    /// Whether the phone may end it.
    killable: bool,
}

fn is_protected(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let own = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));
    PROTECTED.contains(&lower.as_str()) || own.is_some_and(|own| own == lower)
}

impl Tools {
    pub fn new(config_dir: &std::path::Path) -> Self {
        let todo_path = config_dir.join("todos.json");
        let todos = std::fs::read_to_string(&todo_path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        Self { timer: Mutex::new(Timer::new()), todos: Mutex::new(todos), todo_path, listed: Mutex::new(HashMap::new()) }
    }

    fn publish_timer(&self, app: &App) {
        let now = Instant::now();
        let mut timer = self.timer.lock().unwrap();
        timer.tick(now);
        app.hub.publish(TIMER, &timer.state(now));
    }

    fn publish_todos(&self, app: &App) {
        app.hub.publish(TODOS, &*self.todos.lock().unwrap());
    }

    fn save_todos(&self, todos: &[Todo]) -> Result<(), String> {
        let text = serde_json::to_string_pretty(todos).map_err(|e| e.to_string())?;
        std::fs::write(&self.todo_path, text).map_err(|e| format!("saving the to-do list: {e}"))
    }

    fn edit_todos(&self, app: &App, edit: impl FnOnce(&mut Vec<Todo>) -> Result<(), String>) -> Result<(), String> {
        {
            let mut todos = self.todos.lock().unwrap();
            edit(&mut todos)?;
            self.save_todos(&todos)?;
        }
        self.publish_todos(app);
        Ok(())
    }

    /// Runs the action if it belongs to one of these widgets.
    pub fn handle(&self, app: &App, action: &Action) -> Option<Result<(), String>> {
        let now = Instant::now();
        let timer = |change: &dyn Fn(&mut Timer)| {
            change(&mut self.timer.lock().unwrap());
            self.publish_timer(app);
            Some(Ok(()))
        };
        match action {
            Action::TimerToggle { focus, rest } => timer(&|t| t.toggle(now, *focus, *rest)),
            Action::TimerReset => timer(&|t| t.reset()),
            Action::TimerSkip => timer(&|t| t.skip(now)),
            Action::TodoAdd { text } => Some(self.edit_todos(app, |todos| {
                let text: String = text.trim().chars().take(MAX_TODO_LEN).collect();
                if text.is_empty() {
                    return Err("an empty task".to_owned());
                }
                if todos.len() >= MAX_TODOS {
                    return Err("the list is full".to_owned());
                }
                let id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                todos.push(Todo { id, text, done: false });
                Ok(())
            })),
            Action::TodoToggle { id } => Some(self.edit_todos(app, |todos| {
                let todo = todos.iter_mut().find(|t| t.id == *id).ok_or("no such task")?;
                todo.done = !todo.done;
                Ok(())
            })),
            Action::TodoRemove { id } => Some(self.edit_todos(app, |todos| {
                todos.retain(|t| t.id != *id);
                Ok(())
            })),
            Action::TodoClearDone => Some(self.edit_todos(app, |todos| {
                todos.retain(|t| !t.done);
                Ok(())
            })),
            Action::ProcessKill { name } => Some(self.kill(app, name)),
            _ => None,
        }
    }

    fn kill(&self, app: &App, name: &str) -> Result<(), String> {
        if !app.config.read().unwrap().allows_kill() {
            return Err("ending processes is turned off in the config".to_owned());
        }
        if is_protected(name) {
            return Err(format!("{name} is protected"));
        }
        let pids = self.listed.lock().unwrap().get(name).cloned().ok_or("not in the list")?;
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);
        let mut ended = 0;
        for pid in &pids {
            // The pid may have been reused since the list was made.
            if let Some(process) = sys.process(*pid).filter(|p| p.name().to_string_lossy() == name) {
                if process.kill() {
                    ended += 1;
                }
            }
        }
        if ended == 0 {
            return Err(format!("could not end {name}"));
        }
        tracing::info!("ended {ended} × {name}");
        Ok(())
    }
}

pub fn spawn(app: Arc<App>) {
    tokio::spawn(state_loop(app.clone()));
    tokio::spawn(process_loop(app));
}

/// The hub forgets a topic nobody listens to, so the timer and the to-do list
/// are simply re-published while somebody does. Unchanged values are not sent.
async fn state_loop(app: Arc<App>) {
    loop {
        sleep(Duration::from_millis(250)).await;
        // The timer has to advance even when no phone is watching.
        if app.hub.has_subscribers(TIMER) {
            app.tools.publish_timer(&app);
        } else {
            app.tools.timer.lock().unwrap().tick(Instant::now());
        }
        if app.hub.has_subscribers(TODOS) {
            app.tools.publish_todos(&app);
        }
    }
}

async fn process_loop(app: Arc<App>) {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f32;
    let mut sys = System::new();
    let mut warm = false;

    loop {
        sleep(Duration::from_secs(2)).await;
        if !app.hub.has_subscribers(PROCESSES) {
            warm = false;
            continue;
        }
        sys.refresh_processes(ProcessesToUpdate::All, true);
        // CPU usage is a delta: the first sample has nothing to compare to.
        if !warm {
            warm = true;
            continue;
        }

        let mut groups: HashMap<String, (f32, u64, Vec<Pid>)> = HashMap::new();
        for (pid, process) in sys.processes() {
            // Threads show up as processes on some systems; count programs only.
            if process.thread_kind().is_some() {
                continue;
            }
            let group = groups.entry(process.name().to_string_lossy().into_owned()).or_default();
            group.0 += process.cpu_usage() / cores;
            group.1 += process.memory();
            group.2.push(*pid);
        }

        // The busiest by CPU and the biggest by memory; the widget sorts them.
        let mut names: Vec<&String> = groups.keys().collect();
        names.sort_by(|a, b| groups[*b].0.total_cmp(&groups[*a].0));
        let mut chosen: Vec<&String> = names.iter().take(12).copied().collect();
        names.sort_by(|a, b| groups[*b].1.cmp(&groups[*a].1));
        for name in names.iter().take(12) {
            if !chosen.contains(name) {
                chosen.push(name);
            }
        }

        let rows: Vec<ProcessRow> = chosen
            .iter()
            .map(|name| {
                let (cpu, mem, pids) = &groups[*name];
                ProcessRow {
                    name: (*name).clone(),
                    cpu: (cpu * 10.0).round() / 10.0,
                    mem: *mem,
                    count: pids.len(),
                    killable: !is_protected(name),
                }
            })
            .collect();

        *app.tools.listed.lock().unwrap() =
            chosen.iter().map(|name| ((*name).clone(), groups[*name].2.clone())).collect();
        app.hub.publish(PROCESSES, &rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_rolls_into_break_and_break_waits() {
        let start = Instant::now();
        let mut t = Timer::new();
        t.toggle(start, Some(1), Some(2));
        assert!(t.state(start).running);
        assert_eq!(t.state(start).remaining_ms, 60_000);

        // Pausing keeps what is left.
        let later = start + Duration::from_secs(20);
        t.toggle(later, None, None);
        assert!(!t.state(later).running);
        assert_eq!(t.state(later + Duration::from_secs(500)).remaining_ms, 40_000);

        // Focus runs out: the break starts by itself.
        t.toggle(later, None, None);
        let end = later + Duration::from_secs(41);
        t.tick(end);
        let s = t.state(end);
        assert_eq!((s.phase, s.running, s.rounds, s.finished), ("break", true, 1, 1));
        assert_eq!(s.remaining_ms, 120_000);

        // The break runs out: back to focus, waiting.
        let end = end + Duration::from_secs(121);
        t.tick(end);
        let s = t.state(end);
        assert_eq!((s.phase, s.running, s.finished), ("focus", false, 2));
        assert_eq!(s.remaining_ms, 60_000);
    }

    #[test]
    fn system_processes_are_protected() {
        assert!(is_protected("csrss.exe"));
        assert!(is_protected("LSASS.EXE"));
        assert!(!is_protected("notepad.exe"));
    }
}
