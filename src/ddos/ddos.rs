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
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tokio::time;

#[derive(Debug, Clone)]
struct IpStats {
    count: u32,
    window_start: Instant,
    banned_until: Option<Instant>,
}

impl IpStats {
    fn is_expired(&self, now: Instant, window_secs: u64) -> bool {
        now.duration_since(self.window_start).as_secs() > window_secs
    }

    fn is_banned(&self, now: Instant) -> bool {
        matches!(self.banned_until, Some(until) if now < until)
    }

    fn reset_window(&mut self, now: Instant) {
        self.count = 1;
        self.window_start = now;
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn ban(&mut self, ban_duration: Duration) {
        self.banned_until = Some(Instant::now() + ban_duration);
    }

    fn clear_ban(&mut self) {
        self.banned_until = None;
    }
}

#[derive(Clone)]
pub struct DdosConfig {
    pub max_requests: u32,
    pub window_secs: u64,
    pub ban_duration_secs: u64,
    pub block_missing_ua: bool,
    pub blocked_agents: Vec<String>,
    pub cleanup_interval_secs: u64,
    pub max_ip_records: usize,
}

impl Default for DdosConfig {
    fn default() -> Self {
        Self {
            max_requests: 50,
            window_secs: 60,
            ban_duration_secs: 86400,
            block_missing_ua: false,
            blocked_agents: vec!["curl".into(), "wget".into(), "python-requests".into()],
            cleanup_interval_secs: 300, // Clean every 5 minutes
            max_ip_records: 10000,      // Prevent memory exhaustion
        }
    }
}

#[derive(Clone)]
pub struct DdosShield {
    config: DdosConfig,
    ip_records: Arc<RwLock<HashMap<String, IpStats>>>,
}

impl DdosShield {
    pub fn new() -> Self {
        let shield = Self {
            config: DdosConfig::default(),
            ip_records: Arc::new(RwLock::new(HashMap::with_capacity(1024))),
        };
        shield.start_cleanup_task();
        shield
    }

    pub fn builder() -> DdosShieldBuilder {
        DdosShieldBuilder::default()
    }

    fn start_cleanup_task(&self) {
        let ip_records = self.ip_records.clone();
        let config = self.config.clone();
        
        actix_rt::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(config.cleanup_interval_secs));
            loop {
                interval.tick().await;
                Self::cleanup_old_records(&ip_records, &config).await;
            }
        });
    }

    async fn cleanup_old_records(ip_records: &Arc<RwLock<HashMap<String, IpStats>>>, config: &DdosConfig) {
        let mut records = ip_records.write().await;
        let now = Instant::now();
        let ban_duration = Duration::from_secs(config.ban_duration_secs);
        let window_duration = Duration::from_secs(config.window_secs);
        
        // Remove expired entries
        let expired_ips: Vec<String> = records
            .iter()
            .filter(|(_, stats)| {
                // Remove if ban expired AND window is old enough
                let ban_expired = stats.banned_until.map_or(false, |until| now >= until);
                let window_expired = now.duration_since(stats.window_start) > window_duration + ban_duration;
                (ban_expired || stats.banned_until.is_none()) && window_expired
            })
            .map(|(ip, _)| ip.clone())
            .collect();

        for ip in expired_ips {
            records.remove(&ip);
        }

        // Enforce max size limit
        if records.len() > config.max_ip_records {
            let mut entries: Vec<(String, Instant)> = records
                .iter()
                .map(|(ip, stats)| (ip.clone(), stats.window_start))
                .collect();
            entries.sort_by_key(|(_, time)| *time);
            
            let to_remove = records.len() - config.max_ip_records;
            for (ip, _) in entries.into_iter().take(to_remove) {
                records.remove(&ip);
            }
        }
    }

    async fn check_user_agent(&self, req: &ServiceRequest) -> Result<(), Error> {
        let user_agent = req
            .headers()
            .get(header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if self.config.block_missing_ua && user_agent.is_empty() {
            return Err(ErrorForbidden("Blocked: Missing User-Agent"));
        }

        if self.config.blocked_agents.iter().any(|bot| user_agent.contains(bot)) {
            return Err(ErrorForbidden("Blocked: Malicious Actor Detected"));
        }

        Ok(())
    }

    async fn check_rate_limit(&self, ip: &str) -> Result<(), (bool, String)> {
        let mut records = self.ip_records.write().await;
        let now = Instant::now();
        
        let stats = records
            .entry(ip.to_string())
            .or_insert_with(|| IpStats {
                count: 0,
                window_start: now,
                banned_until: None,
            });

        // Check current ban status
        if stats.is_banned(now) {
            return Err((false, "Your IP is banned due to previous abuse.".to_string()));
        }

        // Clear expired ban (should be rare due to cleanup task)
        if stats.banned_until.is_some() {
            stats.clear_ban();
            stats.reset_window(now);
            return Ok(());
        }

        // Check window expiry
        if stats.is_expired(now, self.config.window_secs) {
            stats.reset_window(now);
            return Ok(());
        }

        // Increment counter and check limit
        stats.increment();
        
        if stats.count > self.config.max_requests {
            stats.ban(Duration::from_secs(self.config.ban_duration_secs));
            Err((true, "Rate limit exceeded. Your IP has been temporarily banned.".to_string()))
        } else {
            Ok(())
        }
    }
}

impl Default for DdosShield {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct DdosShieldBuilder {
    config: DdosConfig,
}

impl DdosShieldBuilder {
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

    pub fn cleanup_interval_secs(mut self, secs: u64) -> Self {
        self.config.cleanup_interval_secs = secs;
        self
    }

    pub fn max_ip_records(mut self, max: usize) -> Self {
        self.config.max_ip_records = max;
        self
    }

    pub fn build(self) -> DdosShield {
        let shield = DdosShield {
            config: self.config,
            ip_records: Arc::new(RwLock::new(HashMap::with_capacity(1024))),
        };
        shield.start_cleanup_task();
        shield
    }
}

impl<S, B> Transform<S, ServiceRequest> for DdosShield
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static + Clone,
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
            service,
            shield: self.clone(),
        }))
    }
}

pub struct DdosShieldMiddleware<S> {
    service: S,
    shield: DdosShield,
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
        let shield = self.shield.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // Step 1: Check User-Agent
            if let Err(err) = shield.check_user_agent(&req).await {
                return Err(err);
            }

            // Step 2: Get IP
            let ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown_ip")
                .to_string();

            // Step 3: Check rate limit
            match shield.check_rate_limit(&ip).await {
                Ok(()) => {
                    // Proceed with request
                    service.call(req).await
                }
                Err((triggered, msg)) => {
                    let err = if triggered {
                        ErrorTooManyRequests(msg)
                    } else {
                        ErrorTooManyRequests(msg) // Both are too many requests
                    };
                    Err(err)
                }
            }
        })
    }
}