
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use crate::store::Store;
use crate::model::{Observation as ObsModel, Position as PosModel};

pub mod pb { tonic::include_proto!("atlas4d"); }
use pb::{atlas4_d_server::{Atlas4D, Atlas4DServer}, *};

pub struct Atlas4DService { store: Arc<Mutex<Store>> }
impl Atlas4DService { pub fn new(store: Store) -> Self { Self { store: Arc::new(Mutex::new(store)) } } }

#[tonic::async_trait]
impl Atlas4D for Atlas4DService {
    async fn ingest_many(&self, request: Request<IngestManyRequest>) -> Result<Response<IngestManyResponse>, Status> {
        let req = request.into_inner();
        let mut obs = Vec::new();
        for o in req.observations {
            let t = o.t.parse().map_err(|e: chrono::ParseError| Status::invalid_argument(format!("bad time: {}", e)))?;
            let p = o.pos.as_ref().ok_or(Status::invalid_argument("missing pos"))?;
            let pos = PosModel { lat: p.lat, lon: p.lon, alt_m: p.alt_m };
            let entity_id = uuid::Uuid::parse_str(&o.entity_id).map_err(|e| Status::invalid_argument(format!("bad uuid: {}", e)))?;
            let source = if o.source_json.is_empty() { None } else { serde_json::from_str(&o.source_json).ok() };
            let sigma = if o.sigma_m == 0.0 { None } else { Some(o.sigma_m) };
            obs.push(ObsModel { obs_id: o.obs_id, entity_id, t, pos, quality: o.quality, sigma_m: sigma, source });
        }
        let mut store = self.store.lock().await;
        store.ingest_many(&obs).map_err(|e| Status::internal(format!("ingest error: {}", e)))?;
        Ok(Response::new(IngestManyResponse { ingested: obs.len() as u64 }))
    }

    type QueryNearStream = tokio_stream::wrappers::ReceiverStream<Result<Observation, Status>>;
    async fn query_near(&self, request: Request<QueryNearRequest>) -> Result<Response<Self::QueryNearStream>, Status> {
        let req = request.into_inner();
        let t0 = req.t0.parse().map_err(|e: chrono::ParseError| Status::invalid_argument(format!("bad t0: {}", e)))?;
        let t1 = req.t1.parse().map_err(|e: chrono::ParseError| Status::invalid_argument(format!("bad t1: {}", e)))?;
        let center = PosModel { lat: req.lat, lon: req.lon, alt_m: 0.0 };
        let store = self.store.lock().await;
        let found = store.query_near(&center, req.radius_m, t0, t1).map_err(|e| Status::internal(format!("query error: {}", e)))?;
        let limit = (req.limit as usize).min(found.len());
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        tokio::spawn(async move {
            for o in found.into_iter().take(limit) {
                let ob = Observation {
                    obs_id: o.obs_id,
                    entity_id: o.entity_id.to_string(),
                    t: o.t.to_rfc3339(),
                    pos: Some(Position { lat: o.pos.lat, lon: o.pos.lon, alt_m: o.pos.alt_m }),
                    quality: o.quality,
                    sigma_m: o.sigma_m.unwrap_or(0.0),
                    source_json: o.source.as_ref().map(|v| v.to_string()).unwrap_or_default(),
                };
                if tx.send(Ok(ob)).await.is_err() { break; }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn list_segments(&self, _request: Request<ListSegmentsRequest>) -> Result<Response<ListSegmentsResponse>, Status> {
        let store = self.store.lock().await;
        let segs = store.list_segments().map_err(|e| Status::internal(format!("list error: {}", e)))?;
        let segments = segs.into_iter().map(|(filename, size_bytes, cell, time_bin, binary)| SegmentInfo {
            filename, size_bytes, cell, time_bin, binary
        }).collect();
        Ok(Response::new(ListSegmentsResponse { segments }))
    }
}

pub async fn serve(addr: String, store: Store) -> anyhow::Result<()> {
    use tonic::transport::Server;
    let svc = Atlas4DService::new(store);
    let addr = addr.parse().expect("bind addr");
    println!("gRPC listening on {}", addr);
    Server::builder().add_service(Atlas4DServer::new(svc)).serve(addr).await?;
    Ok(())
}
