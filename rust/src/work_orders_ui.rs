use chrono::TimeZone;
use eframe::egui;
use sc_bookmarks::{
    Bookmark,
    refineries::{Bonus, Load},
    work_orders::{Batch, Job, OutputSource, Queue},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
struct Editor {
    job: Job,
    editing: bool,
    local: bool,
    time: String,
    hours: u32,
    minutes: u32,
    error: String,
}
struct Shared {
    queue: Queue,
    events: Vec<u64>,
    error: Option<String>,
}
pub struct Orders {
    pub queue: Queue,
    path: PathBuf,
    blocked: bool,
    pub message: String,
    editor: Option<Editor>,
    shared: Arc<Mutex<Shared>>,
    worker_started: bool,
    notices: Vec<u64>,
    notice_open: bool,
    sort_station: bool,
    show_collected: bool,
}
impl Orders {
    pub fn new(path: PathBuf) -> Self {
        let (queue, blocked, message) = match Queue::load(&path) {
            Ok(q) => (q, false, String::new()),
            Err(e) => (
                Queue::default(),
                true,
                format!(
                    "Work orders could not load: {e}. Saving is disabled to protect the file; its .json.bak backup is retained."
                ),
            ),
        };
        Self {
            shared: Arc::new(Mutex::new(Shared {
                queue: queue.clone(),
                events: vec![],
                error: None,
            })),
            worker_started: false,
            queue,
            path,
            blocked,
            message,
            editor: None,
            notices: vec![],
            notice_open: false,
            sort_station: false,
            show_collected: false,
        }
    }
    fn commit(&mut self, mut q: Queue) -> bool {
        if self.blocked {
            return false;
        }
        let mut state = self.shared.lock().unwrap();
        // Preserve background-delivered flags if this edit did not intentionally reset them.
        for j in &mut q.jobs {
            if let (Some(old), Some(latest)) = (
                self.queue.jobs.iter().find(|v| v.id == j.id),
                state.queue.jobs.iter().find(|v| v.id == j.id),
            ) {
                if j.completes_utc == old.completes_utc
                    && j.collected_utc == old.collected_utc
                    && j.snooze_until == old.snooze_until
                {
                    if j.advance_notified == old.advance_notified {
                        j.advance_notified = latest.advance_notified;
                    }
                    if j.ready_notified == old.ready_notified {
                        j.ready_notified = latest.ready_notified;
                    }
                    j.snooze_until = latest.snooze_until;
                }
            }
        }
        match q.save(&self.path) {
            Ok(()) => {
                state.queue = q.clone();
                self.queue = q;
                self.message = "Work orders saved locally.".into();
                true
            }
            Err(e) => {
                self.message = format!("Work orders were not saved: {e}");
                false
            }
        }
    }
    pub fn ready(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        self.queue.jobs.iter().filter(|j| j.ready(now)).count()
    }
    pub fn ready_at(&self, id: u64) -> usize {
        let now = chrono::Utc::now().timestamp();
        self.queue
            .jobs
            .iter()
            .filter(|j| j.refinery_id == id && j.ready(now))
            .count()
    }
    pub fn add(&mut self, t: &Bonus, load: &[Load], approach: Option<Bookmark>) {
        if self.blocked {
            return;
        }
        let now = chrono::Utc::now().timestamp();
        self.editor = Some(Editor {
            job: Job {
                id: self.queue.next_id(),
                refinery_id: t.id_terminal,
                refinery: t.terminal_name.clone(),
                name: "Mining load".into(),
                approach,
                method: String::new(),
                fee: None,
                batches: load
                    .iter()
                    .filter(|r| !r.ore.to_lowercase().contains("inert"))
                    .cloned()
                    .map(|input| Batch {
                        input,
                        output_scu: None,
                    })
                    .collect(),
                output_source: OutputSource::Unknown,
                recovery_percent: None,
                created_utc: now,
                completes_utc: now + 3600,
                advance_seconds: 600,
                sound: true,
                collected_utc: None,
                advance_notified: false,
                ready_notified: false,
                snooze_until: None,
            },
            editing: false,
            local: false,
            time: String::new(),
            hours: 1,
            minutes: 0,
            error: String::new(),
        });
    }
    pub fn tick(&mut self, ctx: &egui::Context) -> bool {
        if !self.worker_started && !self.blocked {
            self.worker_started = true;
            let weak = Arc::downgrade(&self.shared);
            let path = self.path.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                loop {
                    let Some(shared) = weak.upgrade() else {
                        break;
                    };
                    let mut notify = false;
                    let mut sound = false;
                    {
                        let mut state = shared.lock().unwrap();
                        let mut candidate = state.queue.clone();
                        let due = candidate.due(chrono::Utc::now().timestamp());
                        if !due.is_empty() {
                            match candidate.save(&path) {
                                Ok(()) => {
                                    state.queue = candidate;
                                    for (id, _) in &due {
                                        if !state.events.contains(id) {
                                            state.events.push(*id);
                                        }
                                    }
                                    notify = true;
                                    sound = due.iter().any(|(_, s)| *s);
                                }
                                Err(e) => {
                                    state.error =
                                        Some(format!("Reminder state could not be saved: {e}"))
                                }
                            }
                        }
                    }
                    drop(shared);
                    if notify {
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Informational,
                        ));
                        if sound {
                            unsafe {
                                let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
                                    windows::Win32::UI::WindowsAndMessaging::MB_OK,
                                );
                            }
                        }
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            });
        }
        {
            let mut state = self.shared.lock().unwrap();
            self.queue = state.queue.clone();
            if let Some(error) = state.error.take() {
                self.message = error;
            }
            for id in state.events.drain(..) {
                if !self.notices.contains(&id) {
                    self.notices.push(id);
                }
                self.notice_open = true;
            }
        }
        let mut open_queue = false;
        if self.notice_open {
            let mut dismiss = false;
            egui::Window::new("Refinery pickup reminders")
                .id(egui::Id::new("refinery_reminder"))
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    let now = chrono::Utc::now().timestamp();
                    ui.label(format!("{} work order reminder(s)", self.notices.len()));
                    egui::ScrollArea::vertical()
                        .max_height(220.)
                        .show(ui, |ui| {
                            for j in self.queue.jobs.iter().filter(|j| {
                                self.notices.contains(&j.id) && j.collected_utc.is_none()
                            }) {
                                ui.label(format!(
                                    "{} / {}: {}",
                                    j.refinery,
                                    j.name,
                                    if j.ready(now) {
                                        "Ready for pickup".into()
                                    } else {
                                        format!("Ready in {}", remaining(j.completes_utc - now))
                                    }
                                ));
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("View work orders").clicked() {
                            open_queue = true;
                            dismiss = true;
                        }
                        if ui.button("Dismiss").clicked() {
                            dismiss = true;
                        }
                    });
                });
            if dismiss {
                self.notice_open = false;
                self.notices.clear();
            }
        }
        open_queue
    }
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(Bookmark, bool)> {
        let now = chrono::Utc::now().timestamp();
        let mut action = None;
        let mut change = None;
        let mut edit = None;
        ui.heading(format!("WORK ORDERS / {} ready for pickup", self.ready()));
        ui.weak("Add jobs from a refinery result. Timers persist across restarts; reminders require this app running or minimized.");
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.sort_station, "Group by refinery");
            ui.checkbox(&mut self.show_collected, "Show collected jobs");
        });
        let mut jobs: Vec<_> = self
            .queue
            .jobs
            .iter()
            .filter(|j| self.show_collected || j.collected_utc.is_none())
            .collect();
        jobs.sort_by(|a, b| {
            if self.sort_station {
                a.refinery
                    .cmp(&b.refinery)
                    .then(a.completes_utc.cmp(&b.completes_utc))
            } else {
                a.completes_utc.cmp(&b.completes_utc)
            }
        });
        if jobs.is_empty() {
            ui.label("No work orders to show. Select a refinery and choose Add refinery job.");
        }
        egui::ScrollArea::vertical()
            .id_salt("work_order_list")
            .show(ui, |ui| {
                for j in jobs {
                    ui.push_id(j.id, |ui| {
                        ui.group(|ui| {
                            ui.strong(format!("{} / {}", j.refinery, j.name));
                            let status = if j.collected_utc.is_some() {
                                "Collected".into()
                            } else if j.ready(now) {
                                "READY FOR PICKUP".into()
                            } else {
                                format!("Refining: {} remaining", remaining(j.completes_utc - now))
                            };
                            ui.colored_label(crate::theme::AMBER, status);
                            ui.label(format!(
                                "Completion: {} (local) · Method: {}",
                                local_time(j.completes_utc),
                                if j.method.is_empty() {
                                    "Unspecified"
                                } else {
                                    &j.method
                                }
                            ));
                            ui.label(match j.output() {
                                Some(s) => format!(
                                    "{}: {s:.2} SCU",
                                    if j.output_source == OutputSource::TerminalQuote {
                                        "Terminal quote"
                                    } else {
                                        "Manual estimate"
                                    }
                                ),
                                None => "Output unknown / incomplete".into(),
                            });
                            if let Some(fee) = j.fee {
                                ui.label(format!("Fee: {fee:.0} aUEC"));
                            }
                            if let Some(s) = j.snooze_until {
                                ui.weak(format!("Reminder snoozed until {}", local_time(s)));
                            }
                            ui.collapsing("Ore batches", |ui| {
                                for b in &j.batches {
                                    ui.label(format!(
                                        "{} · {:.2} raw SCU · quality {} · output {}",
                                        b.input.ore,
                                        b.input.scu,
                                        b.input
                                            .quality
                                            .map(|q| format!("{q:.0}"))
                                            .unwrap_or_else(|| "unspecified".into()),
                                        b.output_scu
                                            .map(|v| format!("{v:.2} SCU"))
                                            .unwrap_or_else(|| "unknown".into())
                                    ));
                                }
                            });
                            ui.horizontal_wrapped(|ui| {
                                if let Some(b) = &j.approach {
                                    if ui.button("Show on map").clicked() {
                                        action = Some((b.clone(), true));
                                    }
                                    if ui.button("Navigate to approach").clicked() {
                                        action = Some((b.clone(), false));
                                    }
                                }
                                if j.collected_utc.is_none() && !self.blocked {
                                    if ui.button("Edit").clicked() {
                                        edit = Some(j.clone());
                                    }
                                    if ui.button("Snooze 15 min").clicked() {
                                        let mut q = self.queue.clone();
                                        q.jobs
                                            .iter_mut()
                                            .find(|x| x.id == j.id)
                                            .unwrap()
                                            .snooze_until = Some(now + 900);
                                        change = Some(q);
                                    }
                                    if ui.button("Mark collected").clicked() {
                                        let mut q = self.queue.clone();
                                        q.jobs
                                            .iter_mut()
                                            .find(|x| x.id == j.id)
                                            .unwrap()
                                            .collected_utc = Some(now);
                                        change = Some(q);
                                    }
                                } else if !self.blocked && ui.button("Undo collected").clicked() {
                                    let mut q = self.queue.clone();
                                    let x = q.jobs.iter_mut().find(|x| x.id == j.id).unwrap();
                                    x.collected_utc = None;
                                    x.ready_notified = false;
                                    change = Some(q);
                                }
                            });
                        });
                    });
                }
            });
        if let Some(q) = change {
            self.commit(q);
        }
        if let Some(job) = edit {
            self.editor = Some(Editor {
                time: local_time(job.completes_utc),
                job,
                editing: true,
                local: true,
                hours: 1,
                minutes: 0,
                error: String::new(),
            });
        }
        action
    }
    pub fn editor(&mut self, ctx: &egui::Context, methods: &[String]) {
        let Some(e) = &mut self.editor else {
            return;
        };
        let mut save = false;
        let mut cancel = false;
        egui::Window::new(if e.editing{"Edit refinery job"}else{"Add refinery job"}).id(egui::Id::new("work_order_editor")).default_width(620.).resizable(true).show(ctx,|ui|{
   egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-150.).max(250.)).show(ui,|ui|{
    ui.strong(&e.job.refinery);ui.label("Job name");ui.add(crate::clear_text::ClearText::singleline(&mut e.job.name));
    egui::ComboBox::from_id_salt("job_method").selected_text(if e.job.method.is_empty(){"Method: unspecified"}else{&e.job.method}).show_ui(ui,|ui|{for m in methods{ui.selectable_value(&mut e.job.method,m.clone(),m);}});
    let mut fee=e.job.fee.is_some();if ui.checkbox(&mut fee,"Record processing fee").changed(){e.job.fee=fee.then_some(0.);}
    if let Some(f)=&mut e.job.fee{ui.add(egui::DragValue::new(f).range(0.0..=1e12).suffix(" aUEC"));}
    ui.separator();ui.horizontal(|ui|{ui.selectable_value(&mut e.local,false,"Finishes in");if ui.selectable_value(&mut e.local,true,"Completion date/time").clicked() && e.time.is_empty(){e.time=local_time(e.job.completes_utc);}});
    if e.local {ui.label("Local PC time: YYYY-MM-DD HH:MM:SS");ui.add(crate::clear_text::ClearText::singleline(&mut e.time));}else{ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut e.hours).range(0..=8760).suffix(" h"));ui.add(egui::DragValue::new(&mut e.minutes).range(0..=59).suffix(" min"));});ui.weak("Duration starts when you save this job.");}
    ui.horizontal(|ui|{let mut advance=e.job.advance_seconds/60;ui.label("Advance notice (minutes; 0 disables)");if ui.add(egui::DragValue::new(&mut advance).range(0..=10080)).changed(){e.job.advance_seconds=advance*60;}ui.checkbox(&mut e.job.sound,"Sound");});
    let old_source=e.job.output_source.clone();
    ui.separator();ui.horizontal_wrapped(|ui|{ui.selectable_value(&mut e.job.output_source,OutputSource::Unknown,"Output unknown");ui.selectable_value(&mut e.job.output_source,OutputSource::TerminalQuote,"Terminal quote");ui.selectable_value(&mut e.job.output_source,OutputSource::ManualEstimate,"Manual estimate");});
    if old_source!=e.job.output_source {for b in &mut e.job.batches{b.output_scu=None;}}
    if e.job.output_source==OutputSource::ManualEstimate {
     ui.weak("Enter your own recovery percentage. This is not a verified game formula or station bonus calculation.");let rate=e.job.recovery_percent.get_or_insert(0.);ui.add(egui::DragValue::new(rate).range(0.0..=100.).suffix("% recovery"));
    }
    ui.weak("Confirm actual ore quantities; bookmark prefill may describe the whole rock. Quality batches stay separate.");
    let mut remove=None;
    for (i,b) in e.job.batches.iter_mut().enumerate(){ui.push_id(i,|ui|{ui.group(|ui|{
     ui.horizontal_wrapped(|ui|{ui.label("Ore");ui.add(crate::clear_text::ClearText::singleline(&mut b.input.ore));if ui.small_button("×").clicked(){remove=Some(i);}});
     ui.horizontal_wrapped(|ui|{ui.label("Raw SCU");ui.add(egui::DragValue::new(&mut b.input.scu).range(0.0..=1e6));let mut quality=b.input.quality.is_some();if ui.checkbox(&mut quality,"Quality").changed(){b.input.quality=quality.then_some(0.);}if let Some(q)=&mut b.input.quality{ui.add(egui::DragValue::new(q).range(0.0..=1000.));}});
     match e.job.output_source {OutputSource::TerminalQuote=>{let mut known=b.output_scu.is_some();if ui.checkbox(&mut known,"Quoted output SCU").changed(){b.output_scu=known.then_some(0.);}if let Some(v)=&mut b.output_scu{ui.add(egui::DragValue::new(v).range(0.0..=1e6));}},OutputSource::ManualEstimate=>{b.output_scu=e.job.recovery_percent.map(|r|b.input.scu*r/100.);ui.label(format!("Estimated output: {:.2} SCU",b.output_scu.unwrap_or(0.)));},OutputSource::Unknown=>{b.output_scu=None;}}
    });});}
    if let Some(i)=remove{e.job.batches.remove(i);}
    if ui.button("Add ore batch").clicked() && e.job.batches.len()<100{e.job.batches.push(Batch{input:Load{ore:"Iron".into(),..Default::default()},output_scu:None});}
    if !e.error.is_empty(){ui.colored_label(egui::Color32::LIGHT_RED,&e.error);}
   });
   ui.horizontal(|ui|{save=ui.button("Save job").clicked();cancel=ui.button("Cancel").clicked();});
  });
        if cancel {
            self.editor = None;
            return;
        }
        if save {
            let e = self.editor.as_mut().unwrap();
            let now = chrono::Utc::now().timestamp();
            let timestamp = if e.local {
                parse_local(&e.time)
            } else {
                Ok(now + i64::from(e.hours) * 3600 + i64::from(e.minutes) * 60)
            };
            match timestamp {
                Err(error) => e.error = error,
                Ok(t) => {
                    let mut job = e.job.clone();
                    if t != job.completes_utc {
                        job.advance_notified = false;
                        job.ready_notified = false;
                        job.snooze_until = None;
                    }
                    job.completes_utc = t;
                    if job.name.trim().is_empty() {
                        job.name = "Mining load".into();
                    }
                    if let Err(error) = job.validate() {
                        e.error = error;
                        return;
                    }
                    let mut q = self.queue.clone();
                    if let Some(old) = q.jobs.iter_mut().find(|j| j.id == job.id) {
                        *old = job;
                    } else {
                        q.jobs.push(job);
                    }
                    if self.commit(q) {
                        self.editor = None;
                    }
                }
            }
        }
    }
}
fn local_time(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}
fn parse_local(s: &str) -> Result<i64, String> {
    let d = chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S")
        .map_err(|_| "Use YYYY-MM-DD HH:MM:SS")?;
    chrono::Local.from_local_datetime(&d).single().map(|t|t.timestamp()).ok_or_else(||"That local time is ambiguous or does not exist due to daylight saving time. Use Finishes in instead.".into())
}
fn remaining(seconds: i64) -> String {
    let s = seconds.max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Orders, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "scbm-orders-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        (Orders::new(dir.join("orders.json")), dir)
    }
    fn add_test(o: &mut Orders) {
        let d = sc_bookmarks::refineries::Data::bundled();
        o.add(d.terminals()[0], &[], None);
        let mut q = Queue::default();
        q.jobs.push(o.editor.as_ref().unwrap().job.clone());
        assert!(o.commit(q));
    }
    fn cleanup(dir: PathBuf) {
        for name in ["orders.json", "orders.json.bak"] {
            let p = dir.join(name);
            if p.exists() {
                std::fs::remove_file(p).unwrap();
            }
        }
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn durable_queue_backup_and_corruption_protection() {
        let (mut o, dir) = fixture();
        add_test(&mut o);
        let mut q = o.queue.clone();
        q.jobs[0].name = "Edited".into();
        assert!(o.commit(q));
        assert!(dir.join("orders.json.bak").exists());
        assert_eq!(Queue::load(&o.path).unwrap().jobs[0].name, "Edited");
        std::fs::write(&o.path, b"corrupt").unwrap();
        let mut broken = Orders::new(o.path.clone());
        assert!(broken.blocked);
        assert!(!broken.commit(Queue::default()));
        assert_eq!(std::fs::read(&o.path).unwrap(), b"corrupt");
        drop(broken);
        drop(o);
        cleanup(dir);
    }
    #[test]
    fn reminders_fire_without_ui_frames() {
        let (mut o, dir) = fixture();
        add_test(&mut o);
        let mut q = o.queue.clone();
        q.jobs[0].completes_utc = chrono::Utc::now().timestamp() - 1;
        q.jobs[0].sound = false;
        assert!(o.commit(q));
        o.tick(&egui::Context::default());
        std::thread::sleep(Duration::from_millis(1200));
        assert!(Queue::load(&o.path).unwrap().jobs[0].ready_notified);
        assert_eq!(o.shared.lock().unwrap().events.len() + o.notices.len(), 1);
        drop(o);
        std::thread::sleep(Duration::from_millis(1100));
        cleanup(dir);
    }
    #[test]
    fn local_time_roundtrip_and_bad_input() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(parse_local(&local_time(now)).unwrap(), now);
        assert!(parse_local("not a date").is_err());
        assert_eq!(remaining(90061), "25:01:01");
    }
}
