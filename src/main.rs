use futures::{self};
use std::{error::Error, sync::Arc, time::Instant};
use tokio::sync::mpsc;

pub struct Report {
    succeeded: i64,
    failed: i64,
    total_requests: i64,
    elapsed: u64,
    transaction_rate: f64,
}

impl Report {
    fn add_report(&mut self, succeed_count: i64, failed_count: i64, total_req_count: i64, el: u64) {
        self.succeeded += succeed_count;
        self.failed += failed_count;
        self.total_requests += total_req_count;
        self.elapsed = el;
    }
}

pub struct Tower {
    // send end
    sender: tokio::sync::mpsc::Sender<Arc<Report>>,
    // receiver end
    receiver: tokio::sync::mpsc::Receiver<Arc<Report>>,
}

impl Tower {
    fn new() -> Tower {
        let (tx, mut rx) = mpsc::channel(100);
        Tower {
            sender: tx,
            receiver: rx,
        }
    }
}

// #[tokio::main]
#[tokio::main(flavor = "current_thread")]
// #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    let report_manager = Tower::new();

    let mut report_receiver = report_manager.receiver;

    let mut report = Report {
        succeeded: 0,
        failed: 0,
        total_requests: 0,
        elapsed: 0,
        transaction_rate: 0.0,
    };

    let tower = tokio::spawn(async move {
        loop {
            match report_receiver.recv().await {
                Some(received_report) => {
                    report.add_report(
                        received_report.succeeded,
                        received_report.failed,
                        received_report.total_requests,
                        received_report.elapsed,
                    );
                }
                None => {
                    report.transaction_rate =
                        (report.total_requests / report.elapsed as i64) as f64;
                        println!(
                            "report: \n\t requests success: {}\n\t requests failed : {} \n\t total requests : {} \n\t elapsed time : {} second(s) \n\t transaction rate: {} requests/s",
                            report.succeeded, report.failed, report.total_requests, report.elapsed, report.transaction_rate
                        );
                    break
                }
            }
        }
    });

    let start = Instant::now();
    load_test(&report_manager.sender).await;
    let elapsed = start.elapsed().as_secs();

    let sender = report_manager.sender.clone();

    let a = tokio::spawn(async move {
        match sender
            .send(Arc::new(Report {
                succeeded: 0,
                failed: 0,
                total_requests: 0,
                elapsed: elapsed,
                transaction_rate: 0.0,
            }))
            .await
        {
            Ok(_) => {}
            Err(_) => {
                println!("err while sending to channel");
            }
        }
    });

    let _ = a.await;

    drop(report_manager.sender);

    tower.await?;
    Ok(())
}

async fn load_test(sender: &tokio::sync::mpsc::Sender<Arc<Report>>) {
    let mut handles: Vec<tokio::task::JoinHandle<()>> = vec![];

    for _i in 0..100 {
        let sender = sender.clone();

        let handle = tokio::spawn(async move {
            match reqwest::get("http://httpbin.org/ip").await {
                Ok(res) => {
                    if res.status() == 200 {
                        match sender
                            .send(Arc::new(Report {
                                succeeded: 1,
                                failed: 0,
                                total_requests: 1,
                                elapsed: 0,
                                transaction_rate: 0.0,
                            }))
                            .await
                        {
                            Ok(_) => {}
                            Err(_) => {
                                println!("err while sending to channel");
                                return;
                            }
                        }
                    } else {
                        match sender
                            .send(Arc::new(Report {
                                succeeded: 0,
                                failed: 1,
                                total_requests: 1,
                                elapsed: 0,
                                transaction_rate: 0.0,
                            }))
                            .await
                        {
                            Ok(_) => {}
                            Err(_) => {
                                println!("err while sending to channel");
                                return;
                            }
                        }
                    }
                }
                Err(_) => {
                    match sender
                        .send(Arc::new(Report {
                            succeeded: 0,
                            failed: 1,
                            total_requests: 1,
                            elapsed: 0,
                            transaction_rate: 0.0,
                        }))
                        .await
                    {
                        Ok(_) => {}
                        Err(_) => {
                            println!("err while sending to channel");
                            return;
                        }
                    }
                }
            }
        });
        handles.push(handle)
    }
    futures::future::join_all(handles).await;
}
