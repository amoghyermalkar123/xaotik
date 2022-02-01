use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{self};
mod tui_backend;
mod types;
use std::num::ParseIntError;
use types::{MachineDetails, Report};
use tui::backend::CrosstermBackend;
use tui::Terminal;
use std::io::{self, Stdout};

pub struct Tower {
    // send end
    sender: tokio::sync::mpsc::Sender<Arc<Report>>,
    // receiver end
    receiver: tokio::sync::mpsc::Receiver<Arc<Report>>,
}

impl Tower {
    fn new() -> Tower {
        let (tx, rx) = mpsc::channel(100);
        Tower {
            sender: tx,
            receiver: rx,
        }
    }
}

use structopt::StructOpt;

/// loadtest the given url with the parameters
#[derive(StructOpt)]
struct Cli {
    /// duration of test
    #[structopt(short = "d", long = "duration", default_value = "25")]
    duration: String,
    /// number of concurrent clients
    #[structopt(short = "c", long = "concurrency", default_value = "10")]
    concurrent_clients: u64,
    /// queries per second
    #[structopt(short = "qps", long = "queries-per-second", default_value = "10")]
    qps: u64,
    /// files to read target urls from
    #[structopt(short = "f", long = "file")]
    file:String,
    #[structopt(short = "u", long = "url")]
    url:String,
}

// #[tokio::main]
// #[tokio::main(flavor = "current_thread")]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), ()> {
    let args = Cli::from_args();
    let test_duration = args.duration.parse::<u64>().unwrap_or(25);

    let readonly_url = Arc::new(args.url);

    load_test(test_duration, args.concurrent_clients, args.qps, readonly_url).await?;
    Ok(())
}

async fn load_test(test_duration: u64, concurrent_clients: u64, qps: u64, url:Arc<String>) -> Result<(), ()> {
    let mut report_manager = Tower::new();

    let csend = report_manager.sender.clone();

    let (tx, rx) = flume::unbounded();
    let workers: u64 = concurrent_clients;
    // load balancers are mapped to OS threads which are scheduled over cpus
    // that are scheduled and managed by tokio (os level scheduling also there). For now 10 threads.

    let load_balancer = (0..workers)
        .map(|_| {
            let sendc = csend.clone();
            let rx = rx.clone();
            // let read_url = &host;
            let host_url = url.clone();

            tokio::spawn(async move {
                while let Ok(()) = rx.recv_async().await {
                    match do_req(host_url.as_str()).await {
                        Ok(request_result) => match sendc.send(request_result).await {
                            Ok(_) => {}
                            Err(_) => {
                                println!("err while sending to channel");
                                return;
                            }
                        },
                        Err(_) => {}
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    let start = Instant::now();
    let dead_line = start + Duration::new(test_duration, 0);

    let mut report = Report::new();

    let tower = tokio::spawn(async move {

        let _ = tui_backend::write_to_t(
            &mut report,
            &mut report_manager.receiver,
            start,
            Duration::new(test_duration, 0),
        )
        .await;
    });

    // qps is queries per second
    let qps = qps;

    let load_gen = tokio::spawn(async move {
        for i in 0.. {
            // println!("{}'th attempt", i);
            if std::time::Instant::now() > dead_line {
                break;
            }
            if tx.send_async(()).await.is_err() {
                println!("GOT ERROR");
                break;
            }
            // waiting for this formula to make more sense in hindsight, i just found it somewhere,
            // this is a shameless copy pasta.
            let sleep_for =
                (start + i as u32 * std::time::Duration::from_secs(1) / qps as u32).into();
            // println!("sleeping for : {:?}", sleep_for);
            tokio::time::sleep_until(sleep_for).await;
        }
    });

    for thread in load_balancer {
        let _ = thread.await;
    }

    let _ = load_gen.await;

    let _ = tower.await;

    Ok(())
}

async fn do_req(host: &str) -> Result<Arc<Report>, ()> {
    let start_of_request = Instant::now();

    let make_request = async {
        match reqwest::get(host).await {
            Ok(res) => {
                if res.status() == 200 {
                    Ok(Arc::new(Report {
                        succeeded: 1,
                        failed: 0,
                        total_requests: 1,
                        elapsed: 0,
                        transaction_rate: 0.0,
                        duration: start_of_request.elapsed(),
                    }))
                } else {
                    Ok(Arc::new(Report {
                        succeeded: 0,
                        failed: 1,
                        total_requests: 1,
                        elapsed: 0,
                        transaction_rate: 0.0,
                        duration: start_of_request.elapsed(),
                    }))
                }
            }
            Err(_) => Ok(Arc::new(Report {
                succeeded: 0,
                failed: 1,
                total_requests: 1,
                elapsed: 0,
                transaction_rate: 0.0,
                duration: start_of_request.elapsed(),
            })),
        }
    };

    tokio::select! {
        res = make_request => {
            res
        }
    }
}
