use prost::Message;
use tonic::codec::ProstCodec;
use crate::error::AppError;

// --- Proto message types (hand-written, matching engine pb/) ---

#[derive(Clone, PartialEq, Message)]
pub struct SubmitJobRequest {
    #[prost(string, tag = "1")]
    pub job_type: String,
    #[prost(string, tag = "2")]
    pub payload_json: String,
    #[prost(int32, tag = "3")]
    pub priority: i32,
    #[prost(int32, tag = "4")]
    pub max_retries: i32,
}

#[derive(Clone, PartialEq, Message)]
pub struct GetJobRequest {
    #[prost(string, tag = "1")]
    pub id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct CancelJobRequest {
    #[prost(string, tag = "1")]
    pub id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct JobResponse {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub job_type: String,
    #[prost(string, tag = "3")]
    pub status: String,
    #[prost(int32, tag = "4")]
    pub attempts: i32,
    #[prost(int32, tag = "5")]
    pub max_retries: i32,
    #[prost(string, tag = "6")]
    pub payload_json: String,
    #[prost(string, tag = "7")]
    pub error: String,
    #[prost(string, tag = "8")]
    pub created_at: String,
    #[prost(string, tag = "9")]
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct CancelJobResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
}

// --- gRPC client ---

pub struct GrpcClient {
    inner: tonic::client::Grpc<tonic::transport::Channel>,
}

impl GrpcClient {
    pub async fn connect(addr: &str) -> Result<Self, AppError> {
        let endpoint =
            tonic::transport::Channel::from_shared(format!("http://{}", addr))
                .map_err(|e| AppError::Other(e.to_string()))?;
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| AppError::EngineUnreachable(format!("{}: {}", addr, e)))?;
        Ok(Self {
            inner: tonic::client::Grpc::new(channel),
        })
    }

    pub async fn submit_job(
        &mut self,
        job_type: String,
        payload_json: String,
        priority: i32,
        max_retries: i32,
    ) -> Result<JobResponse, AppError> {
        self.inner
            .ready()
            .await
            .map_err(|_| AppError::EngineUnreachable("not ready".to_string()))?;
        let path = http::uri::PathAndQuery::from_static(
            "/asyioflow.JobService/SubmitJob",
        );
        let req = SubmitJobRequest {
            job_type,
            payload_json,
            priority,
            max_retries,
        };
        let resp = self
            .inner
            .unary::<_, JobResponse, ProstCodec<SubmitJobRequest, JobResponse>>(
                tonic::Request::new(req),
                path,
                ProstCodec::default(),
            )
            .await
            .map_err(AppError::from)?;
        Ok(resp.into_inner())
    }

    pub async fn get_job(&mut self, id: String) -> Result<JobResponse, AppError> {
        self.inner
            .ready()
            .await
            .map_err(|_| AppError::EngineUnreachable("not ready".to_string()))?;
        let path = http::uri::PathAndQuery::from_static(
            "/asyioflow.JobService/GetJob",
        );
        let resp = self
            .inner
            .unary::<_, JobResponse, ProstCodec<GetJobRequest, JobResponse>>(
                tonic::Request::new(GetJobRequest { id }),
                path,
                ProstCodec::default(),
            )
            .await
            .map_err(AppError::from)?;
        Ok(resp.into_inner())
    }

    pub async fn cancel_job(&mut self, id: String) -> Result<bool, AppError> {
        self.inner
            .ready()
            .await
            .map_err(|_| AppError::EngineUnreachable("not ready".to_string()))?;
        let path = http::uri::PathAndQuery::from_static(
            "/asyioflow.JobService/CancelJob",
        );
        let resp = self
            .inner
            .unary::<_, CancelJobResponse, ProstCodec<CancelJobRequest, CancelJobResponse>>(
                tonic::Request::new(CancelJobRequest { id }),
                path,
                ProstCodec::default(),
            )
            .await
            .map_err(AppError::from)?;
        Ok(resp.into_inner().success)
    }
}
