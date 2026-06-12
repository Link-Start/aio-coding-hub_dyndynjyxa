//! Usage: Apply gateway response chunk plugin hooks before stream accounting.

use crate::gateway::plugins::context::GatewayStreamHookInput;
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use axum::body::Bytes;
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

type PluginChunkFuture =
    Pin<Box<dyn Future<Output = Result<Option<Bytes>, reqwest::Error>> + Send>>;

pub(super) const PLUGIN_STREAM_ERROR_MARKER: &str = ": aio-plugin-error\n";

pub(in crate::gateway) struct PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    pipeline: Arc<GatewayPluginPipeline>,
    db: crate::db::Db,
    trace_id: String,
    sequence: u64,
    pending: Option<PluginChunkFuture>,
}

impl<S> PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        pipeline: Arc<GatewayPluginPipeline>,
        db: crate::db::Db,
        trace_id: String,
    ) -> Self {
        Self {
            upstream,
            pipeline,
            db,
            trace_id,
            sequence: 0,
            pending: None,
        }
    }
}

impl<S> Stream for PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        loop {
            if let Some(mut pending) = this.pending.take() {
                match pending.as_mut().poll(cx) {
                    Poll::Ready(Ok(Some(chunk))) => return Poll::Ready(Some(Ok(chunk))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(None),
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Poll::Pending => {
                        this.pending = Some(pending);
                        return Poll::Pending;
                    }
                }
            }

            let chunk = match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => chunk,
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            this.sequence = this.sequence.saturating_add(1);
            let pipeline = Arc::clone(&this.pipeline);
            let db = this.db.clone();
            let trace_id = this.trace_id.clone();
            let sequence = this.sequence;
            this.pending = Some(Box::pin(async move {
                let input = GatewayStreamHookInput {
                    trace_id: trace_id.clone(),
                    chunk,
                    sequence,
                };
                match pipeline.run_stream_hook(input).await {
                    Ok(output) => {
                        crate::gateway::plugins::audit::persist_gateway_plugin_audit_events(
                            &db,
                            &trace_id,
                            output.audit_events.clone(),
                        );
                        if let Some(blocked) = output.blocked {
                            tracing::warn!(
                                trace_id = %trace_id,
                                status = blocked.status,
                                reason = %blocked.reason,
                                "plugin blocked gateway stream chunk"
                            );
                            return Ok(Some(Bytes::from(format!(
                                "{PLUGIN_STREAM_ERROR_MARKER}event: error\ndata: {{\"error\":\"plugin_blocked\",\"reason\":{}}}\n\n",
                                serde_json::to_string(&blocked.reason)
                                    .unwrap_or_else(|_| "\"Plugin blocked gateway stream\"".to_string())
                            ))));
                        }
                        Ok(Some(output.chunk))
                    }
                    Err(err) => {
                        tracing::warn!(
                            trace_id = %trace_id,
                            error = %err,
                            "plugin stream hook failed"
                        );
                        Ok(Some(Bytes::from(format!(
                            "{PLUGIN_STREAM_ERROR_MARKER}event: error\ndata: {{\"error\":\"plugin_failed\",\"reason\":{}}}\n\n",
                            serde_json::to_string(&err.to_string())
                                .unwrap_or_else(|_| "\"Plugin stream hook failed\"".to_string())
                        ))))
                    }
                }
            }));
        }
    }
}
