use futures::{self};
use std::{error::Error, sync::Arc, time::Instant};
use tokio::sync::mpsc::{self, Sender};
mod tui_backend;
mod types;

use types::Report;

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
                    let reporter = &report;
                    // println!("{}", reporter.total_requests);
                    let _ = tui_backend::write_to_t(reporter).await;
                }
                None => {
                    break;
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
