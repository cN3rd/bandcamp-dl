use anyhow::anyhow;
use http::{Extensions, HeaderMap, StatusCode};
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug, Copy, Clone)]
pub struct Rate {
    num: u64,
    per: Duration,
}

impl Rate {
    pub const fn new(num: u64, per: Duration) -> Self {
        assert!(num > 0);
        assert!(per.as_nanos() > 0);
        Self { num, per }
    }
}

#[derive(Debug)]
struct State {
    until: Instant,
    rem: u64,
}

#[derive(Debug, Clone)]
pub struct RateLimitMiddleware {
    rate: Rate,
    state: Arc<Mutex<State>>,
}

impl RateLimitMiddleware {
    pub fn new(num: u64, per: Duration) -> Self {
        let rate = Rate::new(num, per);
        let state = State {
            until: Instant::now(),
            rem: rate.num,
        };

        Self {
            rate,
            state: Arc::new(Mutex::new(state)),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for RateLimitMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let now = Instant::now();
        let should_sleep = {
            let mut state = self.state.lock().unwrap();

            if now >= state.until {
                state.until = now + self.rate.per;
                state.rem = self.rate.num;
            }

            if state.rem > 0 {
                state.rem -= 1;
                None
            } else {
                Some(state.until - now)
            }
        };

        if let Some(sleep_duration) = should_sleep {
            sleep(sleep_duration).await;
        }

        next.run(req, extensions).await
    }
}

pub struct RetryMiddleware {
    is_waiting: Arc<Mutex<bool>>,
    max_retries: u32,
}

impl RetryMiddleware {
    pub fn new(max_retries: u32) -> Self {
        Self {
            is_waiting: Arc::new(Mutex::new(false)),
            max_retries,
        }
    }

    fn get_retry_after(headers: &HeaderMap) -> Option<Duration> {
        headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
    }
}
#[async_trait::async_trait]
impl Middleware for RetryMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        for n in 0..self.max_retries {
            if *self.is_waiting.lock().unwrap() {
                sleep(Duration::from_millis(100)).await;
                continue;
            }

            let response = next
                .clone()
                .run(req.try_clone().unwrap(), extensions)
                .await?;

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                if let Some(retry_after) = Self::get_retry_after(response.headers()) {
                    *self.is_waiting.lock().unwrap() = true;
                    sleep(retry_after).await;
                    *self.is_waiting.lock().unwrap() = false;
                }
                continue;
            }

            return Ok(response);
        }

        Err(reqwest_middleware::Error::Middleware(anyhow!(
            "Timed out on requests"
        )))
    }
}
