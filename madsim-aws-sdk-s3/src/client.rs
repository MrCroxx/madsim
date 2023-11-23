use aws_types::SdkConfig;
use madsim::net::{Endpoint, Payload};
use std::fmt::Debug;
use std::sync::Arc;

use crate::config::Config;
use crate::error::SdkError;
use crate::server::service::Request;

#[derive(Debug, Clone)]
pub struct Client {
    config: Arc<Config>,
}

impl Client {
    pub fn new(sdk_config: &SdkConfig) -> Self {
        Self::from_conf(sdk_config.into())
    }

    pub fn from_conf(conf: Config) -> Self {
        tracing::debug!(?conf, "new client");
        Self {
            config: Arc::new(conf),
        }
    }

    // operation methods defined in the `operation` mod

    pub(crate) async fn send_request<O: 'static, E: 'static>(
        &self,
        req: Request,
    ) -> Result<O, SdkError<E>> {
        let resp = self.send_request_io(req).await.map_err(|e| {
            SdkError::dispatch_failure(aws_smithy_runtime_api::client::result::ConnectorError::io(
                Box::new(e),
            ))
        })?;
        let resp = *resp.downcast::<Result<O, E>>().expect("failed to downcast");
        resp.map_err(|e| {
            SdkError::service_error(
                e,
                aws_smithy_runtime_api::http::Response::new(
                    aws_smithy_runtime_api::http::StatusCode::try_from(500).unwrap(),
                    aws_smithy_types::body::SdkBody::empty(),
                ),
            )
        })
    }

    async fn send_request_io(&self, req: Request) -> std::io::Result<Payload> {
        let addr = self.config.endpoint_addr;
        let ep = Endpoint::connect(addr).await?;
        let (tx, mut rx) = ep.connect1(addr).await?;
        tx.send(Box::new(req)).await?;
        rx.recv().await
    }
}
