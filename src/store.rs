
use crate::model::{Observation, Position};
use crate::util::{haversine_m, time_bin_5min};
use anyhow::{Result};
use chrono::{DateTime, Utc};
use h3o::{LatLng, Resolution, CellIndex};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};

const MAGIC: &[u8; 5] = b"A4D1"; // simple binary header (magic + version)

pub struct Store { pub root: PathBuf }

impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        if !root.exists() { fs::create_dir_all(&root)?; }
        fs::create_dir_all(root.join("segments"))?;
        fs::create_dir_all(root.join("indices"))?;
        Ok(Self { root })
    }

    fn seg_path(&self, cell: CellIndex, bin: i64) -> PathBuf {
        self.root.join("segments").join(format!("{}_{}.seg", u64::from(cell), bin))
    }

    fn cell_of(pos: &Position) -> CellIndex {
        let ll = LatLng::new(pos.lat, pos.lon).expect("valid lat/lon");
        ll.to_cell(Resolution::Nine)
    }

    pub fn ingest_many(&mut self, obs: &[Observation]) -> Result<()> {
        use std::collections::BTreeMap;
        let mut map: BTreeMap<(CellIndex, i64), Vec<&Observation>> = BTreeMap::new();
        for o in obs {
            let cell = Self::cell_of(&o.pos);
            let bin = time_bin_5min(&o.t);
            map.entry((cell, bin)).or_default().push(o);
        }
        for ((cell, bin), group) in map {
            let path = self.seg_path(cell, bin);
            let new_file = !path.exists() || fs::metadata(&path)?.len() == 0;
            let mut f = fs::OpenOptions::new().create(true).append(true).open(&path)?;
            if new_file { f.write_all(MAGIC)?; }
            for o in group {
                let bytes = bincode::serialize(o)?;
                let len = bytes.len() as u32;
                f.write_all(&len.to_le_bytes())?;
                f.write_all(&bytes)?;
            }
        }
        Ok(())
    }

    pub fn ingest_jsonl(&mut self, input: &Path) -> Result<()> {
        let f = File::open(input)?;
        let r = BufReader::new(f);
        let mut batch = Vec::new();
        for line in r.lines() {
            let line = line?;
            let o: Observation = serde_json::from_str(&line)?;
            batch.push(o);
            if batch.len() >= 10_000 { self.ingest_many(&batch)?; batch.clear(); }
        }
        if !batch.is_empty() { self.ingest_many(&batch)?; }
        Ok(())
    }

    fn read_segment(path: &Path) -> Result<Vec<Observation>> {
        let mut f = File::open(path)?;
        let mut header = [0u8; 5];
        let n = f.read(&mut header)?;
        if n == 5 && &header == MAGIC {
            let mut out = Vec::new();
            loop {
                let mut len_buf = [0u8; 4];
                match f.read_exact(&mut len_buf) {
                    Ok(()) => {},
                    Err(_) => break, // EOF
                }
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                f.read_exact(&mut buf)?;
                let o: Observation = bincode::deserialize(&buf)?;
                out.push(o);
            }
            Ok(out)
        } else {
            // fallback legacy JSONL
            let mut out = Vec::new();
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&header));
            f.read_to_string(&mut s)?;
            let r = BufReader::new(s.as_bytes());
            for line in r.lines() {
                if let Ok(line) = line {
                    if let Ok(o) = serde_json::from_str::<Observation>(&line) {
                        out.push(o);
                    }
                }
            }
            Ok(out)
        }
    }

    pub fn compact_all(&mut self) -> Result<usize> {
        let seg_dir = self.root.join("segments");
        let mut count = 0usize;
        for entry in fs::read_dir(seg_dir)? {
            let entry = entry?;
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("seg") if path.extension().and_then(|e| e.to_str()) not in [Some("seg"), Some("jsonl")] { continue; }if path.extension().and_then(|e| e.to_str()) not in [Some("seg"), Some("jsonl")] { continue; } ext != Some("jsonl") { continue; }
            let obs = Self::read_segment(&path)?;
            if obs.is_empty() { continue; }
            let mut map: HashMap<u64, usize> = HashMap::new();
            let mut dedup = Vec::new();
            for o in obs {
                if let Some(idx) = map.get(&o.obs_id).cloned() { dedup[idx] = o; }
                else { map.insert(o.obs_id, dedup.len()); dedup.push(o); }
            }
            dedup.sort_by_key(|o| o.t);
            let tmp = path.with_extension("seg.tmp");
            let mut f = fs::OpenOptions::new().create(true).write(true).truncate(true).open(&tmp)?;
            f.write_all(MAGIC)?;
            for o in dedup {
                let bytes = bincode::serialize(&o)?;
                f.write_all(&(bytes.len() as u32).to_le_bytes())?;
                f.write_all(&bytes)?;
            }
            drop(f);
            fs::rename(tmp, &path)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn list_segments(&self) -> Result<Vec<(String, u64, String, i64, bool)>> {
        let seg_dir = self.root.join("segments");
        let mut out = Vec::new();
        for entry in fs::read_dir(seg_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() { continue; }
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            let size = fs::metadata(&path)?.len();
            let binary = match path.extension().and_then(|e| e.to_str()) {
                Some("seg") => true,
                Some("jsonl") => false,
                _ => false,
            };
            let stem = path.file_stem().unwrap().to_string_lossy();
            let parts: Vec<&str> = stem.split('_').collect();
            let (cell, bin) = if parts.len() >= 2 {
                (parts[0].to_string(), parts[1].parse::<i64>().unwrap_or(0))
            } else { ("?".into(), 0) };
            out.push((fname, size, cell, bin, binary));
        }
        out.sort_by_key(|v| (v.2.clone(), v.3));
        Ok(out)
    }

    pub fn rebuild_indices(&mut self) -> Result<()> { Ok(()) }

    pub fn query_near(
        &self, center: &Position, r_m: f64, t0: DateTime<Utc>, t1: DateTime<Utc>
    ) -> Result<Vec<Observation>> {
        let center_cell = Self::cell_of(center);
        let k = ((r_m / 170.0).ceil() as u32).min(8);
        let mut cells = HashSet::new();
        for ring_cell in center_cell.k_ring(k) { cells.insert(ring_cell); }

        let mut bins = Vec::new();
        let mut b = time_bin_5min(&t0);
        let b_end = time_bin_5min(&t1) + 5*60;
        while b <= b_end { bins.push(b); b += 5*60; }

        let mut out = Vec::new();
        for cell in cells {
            for bin in &bins {
                let path = self.seg_path(cell, *bin);
                let mut tried = false;
                if path.exists() {
                    tried = true;
                    let obs = Self::read_segment(&path)?;
                    for o in obs {
                        if o.t < t0 || o.t > t1 { continue; }
                        let d = haversine_m(center.lat, center.lon, o.pos.lat, o.pos.lon);
                        if d <= r_m { out.push(o); }
                    }
                }
                if !tried {
                    let legacy = self.root.join("segments").join(format!("{}_{}.jsonl", u64::from(cell), *bin));
                    if legacy.exists() {
                        let obs = Self::read_segment(&legacy)?;
                        for o in obs {
                            if o.t < t0 || o.t > t1 { continue; }
                            let d = haversine_m(center.lat, center.lon, o.pos.lat, o.pos.lon);
                            if d <= r_m { out.push(o); }
                        }
                    }
                }
            }
        }
        out.sort_by_key(|o| o.t);
        Ok(out)
    }
}
