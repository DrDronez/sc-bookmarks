use eframe::egui;
use sc_bookmarks::{
    Bookmark,
    refineries::{self, Data, Load},
    routes::Catalog,
};
use std::{path::PathBuf, sync::mpsc};
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Preferences {
    load: Vec<Load>,
    preferred: Option<u64>,
    origin: Option<Bookmark>,
    estimate: bool,
    all_systems: bool,
    max_mkm: f64,
}
pub struct Refineries {
    pub orders: crate::work_orders_ui::Orders,
    pub queue_tab: bool,
    pub data: Data,
    prefs: Preferences,
    path: PathBuf,
    selected: Option<u64>,
    sort: usize,
    use_live: bool,
    status: String,
    pending: Option<mpsc::Receiver<Result<Data, String>>>,
    last_refresh: Option<std::time::Instant>,
}
impl Refineries {
    pub fn new(path: PathBuf) -> Self {
        let cache = path.with_file_name("refineries-cache.json");
        let data = std::fs::metadata(&cache)
            .ok()
            .filter(|m| m.len() < 2_000_000)
            .and_then(|_| std::fs::read(&cache).ok())
            .and_then(|b| serde_json::from_slice::<Data>(&b).ok())
            .filter(|d| d.validate().is_ok())
            .unwrap_or_else(Data::bundled);
        let prefs = std::fs::read(&path)
            .ok()
            .filter(|b| b.len() < 500_000)
            .and_then(|b| serde_json::from_slice::<Preferences>(&b).ok())
            .filter(|p| {
                p.load.len() <= 100
                    && p.load.iter().all(|r| {
                        r.ore.len() <= 100 && r.scu.is_finite() && r.scu >= 0. && r.scu <= 1e6
                    })
                    && p.origin.as_ref().is_none_or(|b| b.validate().is_ok())
            })
            .unwrap_or_else(|| Preferences {
                load: vec![Load {
                    ore: "Iron".into(),
                    scu: 0.,
                    quality: None,
                }],
                max_mkm: 30.,
                ..Default::default()
            });
        Self {
            orders: crate::work_orders_ui::Orders::new(
                path.with_file_name("refinery-work-orders.json"),
            ),
            queue_tab: false,
            data,
            prefs,
            path,
            selected: None,
            sort: 0,
            use_live: true,
            status: "Offline-ready UEX snapshot. Refresh downloads public data only.".into(),
            pending: None,
            last_refresh: None,
        }
    }
    fn save(&mut self) {
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.prefs) {
            if let Err(e) = std::fs::write(&self.path, bytes) {
                self.status = format!("Could not save refinery preferences: {e}");
            }
        }
    }
    pub fn prefill(&mut self, b: &Bookmark) {
        self.prefs.load = refineries::from_bookmark(b);
        self.prefs.origin = Some(b.clone());
        self.prefs.estimate = true;
        self.use_live = false;
        self.save();
    }
    fn refresh(&mut self, ctx: egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.last_refresh = Some(std::time::Instant::now());
        std::thread::spawn(move || {
            let result = (|| {
                let mut payload = serde_json::Map::new();
                for endpoint in ["refineries_yields", "refineries_methods"] {
                    let mut cmd = std::process::Command::new("curl.exe");
                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000);
                    }
                    let output = cmd
                        .args([
                            "--fail",
                            "--silent",
                            "--show-error",
                            "--connect-timeout",
                            "8",
                            "--max-time",
                            "20",
                            "--max-filesize",
                            "2000000",
                            "--proto",
                            "=https",
                            "--user-agent",
                            "SCBookmarks",
                            "--url",
                            &format!("https://api.uexcorp.uk/2.0/{endpoint}"),
                        ])
                        .output()
                        .map_err(|e| format!("Download unavailable: {e}"))?;
                    if !output.status.success() {
                        return Err("UEX download failed; keeping existing data.".into());
                    }
                    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
                        .map_err(|_| "Invalid API response")?;
                    if value["status"] != "ok" {
                        return Err("UEX returned an error; keeping existing data.".into());
                    }
                    payload.insert(endpoint.into(), value["data"].clone());
                }
                let data: Data =
                    serde_json::from_value(payload.into()).map_err(|_| "Unexpected API schema")?;
                data.validate()?;
                Ok(data)
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        live: Option<Bookmark>,
        bookmark: Option<&Bookmark>,
    ) -> Option<(Bookmark, bool)> {
        if let Some(result) = self.pending.as_ref().and_then(|r| r.try_recv().ok()) {
            self.pending = None;
            match result {
                Ok(data) => {
                    let cache = self.path.with_file_name("refineries-cache.json");
                    let temp = cache.with_extension("tmp");
                    let saved = serde_json::to_vec(&data)
                        .map_err(|e| e.to_string())
                        .and_then(|b| std::fs::write(&temp, b).map_err(|e| e.to_string()))
                        .and_then(|_| std::fs::rename(&temp, &cache).map_err(|e| e.to_string()));
                    self.data = data;
                    self.status = match saved {
                        Ok(()) => "Public data updated and cached for offline use.".into(),
                        Err(e) => format!("Data updated for this session; cache failed: {e}"),
                    };
                }
                Err(e) => self.status = e,
            }
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.queue_tab, false, "Find a refinery");
            ui.selectable_value(
                &mut self.queue_tab,
                true,
                format!("Work orders / {} ready", self.orders.ready()),
            );
        });
        if self.queue_tab {
            return self.orders.show(ui);
        }
        let mut action = None;
        let mut changed = false;
        ui.heading("REFINERIES");
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to("Data: UEX", "https://uexcorp.space/mining/refineries/");
            let ready = self.pending.is_none()
                && self.last_refresh.is_none_or(|t| t.elapsed().as_secs() > 60);
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(if self.pending.is_some() {
                        "Refreshing..."
                    } else {
                        "Refresh data"
                    }),
                )
                .clicked()
            {
                self.refresh(ui.ctx().clone());
            }
            if ui
                .add_enabled(
                    bookmark.is_some_and(|b| b.metadata.contains_key("mining")),
                    egui::Button::new("Use selected mining bookmark"),
                )
                .clicked()
            {
                self.prefill(bookmark.unwrap());
            }
        });
        ui.weak(&self.status);
        ui.weak("Reported bonuses, not final yield or profit. Missing reports stay unknown. Confirm quotes at the terminal.");
        egui::ScrollArea::vertical().id_salt("refinery_body").show(ui,|ui|{
   ui.horizontal_wrapped(|ui|{
    changed|=ui.selectable_value(&mut self.prefs.estimate,false,"Actual load (manual)").changed();
    changed|=ui.selectable_value(&mut self.prefs.estimate,true,"Rock estimate").changed();
    if ui.button("Add ore").clicked() && self.prefs.load.len()<100 {self.prefs.load.push(Load{ore:"Iron".into(),..Default::default()});changed=true;}
   });
   if self.prefs.estimate {ui.weak("Rock composition is not extracted cargo. Check quantities before deciding where to refine.");}
   let ores:std::collections::BTreeSet<_>=self.data.refineries_yields.iter().map(|b|refineries::ore_key(&b.commodity_name)).collect();
   let mut remove=None;
   for (i,row) in self.prefs.load.iter_mut().enumerate(){ui.push_id(i,|ui|{ui.horizontal_wrapped(|ui|{
    egui::ComboBox::from_id_salt("ore").selected_text(&row.ore).show_ui(ui,|ui|{for ore in &ores {changed|=ui.selectable_value(&mut row.ore,ore.clone(),ore).changed();}});
    ui.label("SCU");changed|=ui.add(egui::DragValue::new(&mut row.scu).range(0.0..=1000000.).speed(0.1)).changed();
    let mut quality=row.quality.is_some();if ui.checkbox(&mut quality,"Quality").changed(){row.quality=quality.then_some(0.);changed=true;}
    if let Some(q)=&mut row.quality {changed|=ui.add(egui::DragValue::new(q).range(0.0..=1000.)).changed();}
    if ui.small_button("×").clicked(){remove=Some(i);}
   });});}
   if let Some(i)=remove {self.prefs.load.remove(i);changed=true;}
   ui.separator();
   ui.horizontal_wrapped(|ui|{
    ui.selectable_value(&mut self.use_live,true,"From current position");
    ui.selectable_value(&mut self.use_live,false,"From bookmark");
    if ui.add_enabled(bookmark.is_some(),egui::Button::new("Set origin to selected bookmark")).clicked(){self.prefs.origin=bookmark.cloned();self.use_live=false;changed=true;}
    changed|=ui.checkbox(&mut self.prefs.all_systems,"Include other systems").changed();
   });
   let origin=if self.use_live {live.as_ref()}else{self.prefs.origin.as_ref()};
   if let Some(o)=origin {ui.label(format!("Origin: {} / {}",o.name,o.system));} else {ui.colored_label(crate::theme::AMBER,"Origin unavailable. Select a bookmark or start capture; distances and same-system filtering need an origin.");}
   ui.horizontal_wrapped(|ui|{
    egui::ComboBox::from_id_salt("refinery_sort").selected_text(["Sort: Closest","Sort: Best reported bonus","Sort: Best bonus within range"][self.sort]).show_ui(ui,|ui|{for (i,s) in ["Closest","Best reported bonus","Best bonus within range"].iter().enumerate(){ui.selectable_value(&mut self.sort,i,*s);}});
    ui.label("Range (million km)");changed|=ui.add(egui::DragValue::new(&mut self.prefs.max_mkm).range(0.1..=1000.)).changed();
   });
   let mut rows:Vec<_>=self.data.terminals().into_iter().filter_map(|t|{
    let approach=refineries::approach(t,catalog);let distance=origin.and_then(|o|approach.as_ref().and_then(|p|refineries::distance(o,p)));
    if !self.prefs.all_systems && origin.is_some_and(|o|o.system!=t.star_system_name){return None;}
    if self.sort==2 && distance.is_none_or(|d|d>self.prefs.max_mkm*1e6){return None;}
    Some((t,approach,distance,self.data.score(t.id_terminal,&self.prefs.load)))
   }).collect();
   rows.sort_by(|a,b|{if self.sort==0 {a.2.unwrap_or(f64::INFINITY).total_cmp(&b.2.unwrap_or(f64::INFINITY))}else{b.3.unwrap_or(f64::NEG_INFINITY).total_cmp(&a.3.unwrap_or(f64::NEG_INFINITY)).then_with(||a.2.unwrap_or(f64::INFINITY).total_cmp(&b.2.unwrap_or(f64::INFINITY)))}});
   ui.weak("SCU-weighted bonus compares reported ore bonuses only; quality is retained separately. Distances are straight-line to approach points, not travel times.");
   if rows.is_empty(){ui.label("No refineries match this range and system selection.");}
   for (t,approach,distance,score) in rows {
    ui.group(|ui|{
     let title=format!("{}{} / {}",if self.prefs.preferred==Some(t.id_terminal){"★ "}else{""},t.terminal_name,t.star_system_name);
     if ui.selectable_label(self.selected==Some(t.id_terminal),title).clicked(){self.selected=Some(t.id_terminal);}
     ui.horizontal_wrapped(|ui|{
      ui.label(match score {Some(s)=>format!("Weighted bonus: {s:+.2}%"),None=>"Weighted bonus: unknown / incomplete load data".into()});
      ui.label(match distance {Some(d)=>format!("{:.2} million km",d/1e6),None if origin.is_some()=>"System jump required / distance unavailable".into(),_=>"Distance unavailable".into()});
      let report=chrono::DateTime::from_timestamp(self.data.refineries_yields.iter().filter(|b|b.id_terminal==t.id_terminal).map(|b|b.date_modified).min().unwrap_or(0),0).map(|d|d.format("%Y-%m-%d").to_string()).unwrap_or_else(||"Unknown".into());ui.weak(format!("Oldest bonus report: {report}"));
     });
     if self.selected==Some(t.id_terminal){
      if ui.button("Add refinery job").clicked(){self.orders.add(t,&self.prefs.load,approach.clone());}
      for row in self.prefs.load.iter().filter(|r|r.scu>0.){ui.label(format!("{} · {:.2} SCU · quality {} · {}",row.ore,row.scu,row.quality.map(|q|format!("{q:.0}")).unwrap_or_else(||"unspecified".into()),self.data.bonus(t.id_terminal,&row.ore).map(|b|format!("{:+.0}% reported bonus",b.value)).unwrap_or_else(||"No bonus report".into())));}
      if let Some(b)=approach {ui.label(format!("Approach: {}. Use the in-game starmap for final docking/landing.",b.name));ui.horizontal(|ui|{if ui.button("Show on map").clicked(){action=Some((b.clone(),true));}if ui.button("Navigate to approach").clicked(){action=Some((b.clone(),false));}});}
      if ui.button(if self.prefs.preferred==Some(t.id_terminal){"Clear preferred refinery"}else{"Set preferred refinery"}).clicked(){self.prefs.preferred=if self.prefs.preferred==Some(t.id_terminal){None}else{Some(t.id_terminal)};changed=true;}
     }
    });
   }
   ui.collapsing("Refining methods: relative tradeoffs",|ui|{ui.weak("Community ratings; these are not job quotes. Check the current terminal before committing.");for m in &self.data.refineries_methods{let r=|n:u8|["Unknown","Low","Medium","High"][n.min(3) as usize];ui.label(format!("{} · Yield {} · Cost {} · Speed {}",m.name,r(m.rating_yield),r(m.rating_cost),r(m.rating_speed)));}});
  });
        if changed {
            self.save();
        }
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_cache_falls_back_and_preferences_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "scbm-refinery-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("refineries-cache.json"), b"broken").unwrap();
        let path = dir.join("preferences.json");
        let mut view = Refineries::new(path.clone());
        view.data.validate().unwrap();
        view.prefs.preferred = Some(755);
        view.prefs.load = vec![Load {
            ore: "Gold".into(),
            scu: 8.,
            quality: Some(521.),
        }];
        view.save();
        let restored = Refineries::new(path);
        assert_eq!(restored.prefs.preferred, Some(755));
        assert_eq!(restored.prefs.load[0].quality, Some(521.));
        // These are uniquely named test-owned files, never application user data.
        std::fs::remove_file(dir.join("preferences.json")).unwrap();
        std::fs::remove_file(dir.join("refineries-cache.json")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
