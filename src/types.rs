pub struct Report {
    pub succeeded: i64,
    pub failed: i64,
    pub total_requests: i64,
    pub elapsed: u64,
    pub transaction_rate: f64,
}

impl Report {
    pub fn add_report(
        &mut self,
        succeed_count: i64,
        failed_count: i64,
        total_req_count: i64,
        el: u64,
    ) {
        self.succeeded += succeed_count;
        self.failed += failed_count;
        self.total_requests += total_req_count;
        self.elapsed = el;
    }
}
