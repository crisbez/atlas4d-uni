
use anyhow::Result;
use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc, Duration};
use std::path::PathBuf;
mod model; mod store; mod util; mod server;
use model::{Observation, Position}; use store::Store;

#[derive(Parser, Debug)]
#[command(name = "atlas4d-uni")]
#[command(about = "Atlas4D Unique – spatiotemporal DB prototype (gRPC + binary segments)")]
struct Cli {
    #[arg(short, long, default_value = "./atlas4d-data")] data_dir: PathBuf,
    #[command(subcommand)] command: Commands,
}
#[derive(Subcommand, Debug)]
enum Commands {
    DemoIngest { #[arg(long, default_value_t = 10)] minutes: i64 },
    IngestFile { #[arg(short, long)] input: PathBuf },
    QueryNear { #[arg(long)] lat: f64, #[arg(long)] lon: f64, #[arg(long, default_value_t = 50.0)] radius_m: f64, #[arg(long)] t0: String, #[arg(long)] t1: String, #[arg(long, default_value_t = 50)] limit: usize },
    BuildIndex,
    Compact,
    Serve { #[arg(long, default_value = "0.0.0.0:50051")] addr: String },
}
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut store = Store::open(cli.data_dir.clone())?;
    match cli.command {
        Commands::DemoIngest { minutes } => {
            let now = Utc::now(); let start = now - Duration::minutes(minutes);
            let e1 = uuid::Uuid::new_v4(); let e2 = uuid::Uuid::new_v4(); let mut obs = Vec::new();
            for i in 0..(minutes*60/5) {
                let t = start + Duration::seconds(i*5);
                obs.push(Observation::new(e1, t, Position { lat: 42.494 + 0.00005*(i as f64), lon: 27.470 + 0.00007*(i as f64), alt_m: 10.0 }, Some(1.5), 0.95, None));
                obs.push(Observation::new(e2, t, Position { lat: 42.500 - 0.00005*(i as f64), lon: 27.480 - 0.00006*(i as f64), alt_m: 12.0 }, Some(2.0), 0.92, None));
            }
            store.ingest_many(&obs)?; println!("Demo data ingested: {} observations", obs.len());
        }
        Commands::IngestFile { input } => { store.ingest_jsonl(&input)?; println!("File ingested: {}", input.display()); }
        Commands::QueryNear { lat, lon, radius_m, t0, t1, limit } => {
            let t0: DateTime<Utc> = t0.parse()?; let t1: DateTime<Utc> = t1.parse()?;
            let center = Position { lat, lon, alt_m: 0.0 }; let found = store.query_near(&center, radius_m, t0, t1)?;
            println!("Found {} observations:", found.len());
            for o in found.iter().take(limit) { println!("{}; entity={}; t={}; lat={:.6}, lon={:.6}, alt={:.1} m", o.obs_id, o.entity_id, o.t, o.pos.lat, o.pos.lon, o.pos.alt_m); }
        }
        Commands::BuildIndex => { store.rebuild_indices()?; println!("Indices rebuilt."); }
        Commands::Compact => { let n = store.compact_all()?; println!("Compacted {} segment(s).", n); }
        Commands::Serve { addr } => { server::serve(addr, store).await?; }
    } Ok(())
}
