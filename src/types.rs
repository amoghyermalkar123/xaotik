use std::time::Duration;

pub struct Report {
    pub succeeded: i64,
    pub failed: i64,
    pub total_requests: i64,
    pub elapsed: u64,
    pub transaction_rate: f64,
    pub duration: Duration,
}

impl Report {
    pub fn new() -> Self {
        Report {
            succeeded: 0,
            failed: 0,
            total_requests: 0,
            elapsed: 0,
            transaction_rate: 0.0,
            duration: Duration::new(0, 0),
        }
    }
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

pub struct MachineDetails {
    pub ssid: String,
    pub tx_bitrate: f32,
    pub rx_bitrate: f32,
    pub avg_signal: u8,
    pub frequency: u32,
}

impl MachineDetails {
    pub fn new() -> Self {
        MachineDetails {
            ssid: "".to_string(),
            tx_bitrate: 0.0,
            rx_bitrate: 0.0,
            avg_signal: 0,
            frequency: 0,
        }
    }
}
