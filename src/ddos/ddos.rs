// ddos.rs
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorForbidden, ErrorTooManyRequests},
    http::header,
    Error,
};
use futures_util::future::LocalBoxFuture;
use std::{
    collections::HashMap,
    future::ready,
    rc::Rc,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct IpStats {
    count: u32,
    window_start: Instant,
    banned_until: Option<Instant>,
}

#[derive(Clone)]
pub struct DdosConfig {
    pub max_requests: u32,
    pub window_secs: u64,
    pub ban_duration_secs: u64,
    pub block_missing_ua: bool,
    pub blocked_agents: Vec<String>,
}

pub struct DdosShield {
    config: DdosConfig,
    ip_records: Arc<RwLock<HashMap<String, IpStats>>>,
}

impl DdosShield {
    /// Initializes the framework with secure defaults
    pub fn builder() -> Self {
        Self {
            config: DdosConfig {
                max_requests: 50,          // 50 requests
                window_secs: 60,           // per 60 seconds
                ban_duration_secs: 86400,  // 24 Hour ban if breached
                block_missing_ua: false,
                blocked_agents: vec!["curl".into()],
            },
            ip_records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn max_requests(mut self, reqs: u32) -> Self {
        self.config.max_requests = reqs;
        self
    }

    pub fn window_secs(mut self, secs: u64) -> Self {
        self.config.window_secs = secs;
        self
    }

    pub fn ban_duration_secs(mut self, secs: u64) -> Self {
        self.config.ban_duration_secs = secs;
        self
    }

    pub fn block_agent(mut self, agent: &str) -> Self {
        self.config.blocked_agents.push(agent.to_lowercase());
        self
    }

    pub fn allow_missing_ua(mut self, allow: bool) -> Self {
        self.config.block_missing_ua = !allow;
        self
    }
}

impl<S, B> Transform<S, ServiceRequest> for DdosShield
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DdosShieldMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(DdosShieldMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
            ip_records: self.ip_records.clone(),
        }))
    }
}

pub struct DdosShieldMiddleware<S> {
    service: Rc<S>,
    config: DdosConfig,
    ip_records: Arc<RwLock<HashMap<String, IpStats>>>,
}

impl<S, B> Service<ServiceRequest> for DdosShieldMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // --- 1. WAF Security: Validate User-Agent ---
        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if self.config.block_missing_ua && user_agent.is_empty() {
            return Box::pin(ready(Err(ErrorForbidden("Blocked: Missing User-Agent"))));
        }

        if self.config.blocked_agents.iter().any(|bot| user_agent.contains(bot)) {
            return Box::pin(ready(Err(ErrorForbidden("Blocked: Malicious Actor Detected"))));
        }

        // --- 2. IP Rate Limiting & Banning ---
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown_ip")
            .to_string();

        let mut is_banned = false;
        let mut triggered_ban = false;

        {
            let mut records = self.ip_records.write().unwrap();
            let now = Instant::now();
            let stats = records.entry(ip).or_insert(IpStats {
                count: 0,
                window_start: now,
                banned_until: None,
            });

            if let Some(banned_time) = stats.banned_until {
                if now < banned_time {
                    is_banned = true;
                } else {
                    stats.banned_until = None;
                    stats.count = 1;
                    stats.window_start = now;
                }
            } else {
                if now.duration_since(stats.window_start).as_secs() > self.config.window_secs {
                    stats.count = 1;
                    stats.window_start = now;
                } else {
                    stats.count += 1;
                    if stats.count > self.config.max_requests {
                        stats.banned_until = Some(now + Duration::from_secs(self.config.ban_duration_secs));
                        triggered_ban = true;
                        is_banned = true;
                    }
                }
            }
        }

        if is_banned {
            let msg = if triggered_ban {
                "Rate limit exceeded. Your IP has been temporarily banned."
            } else {
                "Your IP is banned due to previous abuse."
            };
            return Box::pin(ready(Err(ErrorTooManyRequests(msg))));
        }

        // --- 3. Pass to the next service ---
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
