use crate::Report;
use crossterm::style::Stylize;
use crossterm::{
    style::{Print, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use std::error::Error;
use std::io::stdout;
use std::io::Write;
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::{self, Receiver};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Corner, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{
    BarChart, Block, Borders, Chart, Dataset, Gauge, List, ListItem, ListState, Paragraph,
    Sparkline, Table,
};
use tui::Terminal;

enum Number {
    Int(i64),
    Float(f64),
}

struct RequestWrapper<'a> {
    events: Vec<(&'a str, Number)>,
}

impl<'a> RequestWrapper<'a> {
    fn new(totreq: Number, s: Number, f: Number, tr: Number) -> RequestWrapper<'a> {
        RequestWrapper {
            events: vec![
                ("Total Requests", totreq),
                ("Succeeded", s),
                ("Failed", f),
                ("Transaction Rate", tr),
            ],
        }
    }
}

pub async fn write_to_t(
    report: &mut Report,
    report_receiver: &mut Receiver<Arc<Report>>,
    test_started_at: Instant,
    total_duration_for_test: Duration,
) -> Result<(), Box<dyn Error>> {
    use std::fs::File;

    let mut f = match File::create("debug.txt") {
        Ok(file) => file,
        _ => panic!(),
    };

    let mut terminal = {
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)?
    };

    let mut durations: Vec<std::time::Duration> = Vec::new();
    terminal.clear()?;

    loop {
        match report_receiver.recv().await {
            Some(received_report) => {
                report.add_report(
                    received_report.succeeded,
                    received_report.failed,
                    received_report.total_requests,
                    received_report.elapsed,
                );
                let _ = write!(
                    f,
                    "{:?}    {:?} \n",
                    received_report.duration,
                    received_report.duration.as_secs_f64()
                );

                durations.push(received_report.duration);
                let dur_copy = durations.clone();
                // TODO: ponder over efficiency
                // let mut dur_collection = dur_copy
                // .iter()
                // .map(|dur| dur.as_secs_f64())
                // .collect::<Vec<_>>();
                // println!("{:?}", dur_collection);
                // println!("=================================")
                draw(
                    &mut terminal,
                    report,
                    dur_copy,
                    test_started_at,
                    total_duration_for_test,
                )?;
            }
            None => {
                std::thread::sleep(std::time::Duration::from_secs(12));
                terminal.clear()?;
                break;
            }
        }
    }
    Ok(())
}

fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    report: &Report,
    durations: Vec<Duration>,
    start: Instant,
    total_test_time: Duration,
) -> Result<(), Box<dyn Error>> {
    terminal.draw(|f| {
        let row4 = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(8),
                    Constraint::Length(10 as u16 + 2),
                    Constraint::Percentage(40),
                ]
                .as_ref(),
            )
            .split(f.size());

        // let progress = (report.total_requests * 100) / 1000;

        let now = std::time::Instant::now();

        let progress = ((now - start).as_secs_f64() / total_test_time.as_secs_f64())
            .max(0.0)
            .min(1.0);

        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(row4[1]);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(row4[2]);

        let t = Duration::from(std::time::Duration::from_secs(
            (now - start).as_secs_f64() as u64
        ));

        let gauge_label = format!("{:?} / {:?}", t, Duration::from(total_test_time));

        let gauge = Gauge::default()
            .block(Block::default().title("Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(tui::style::Color::Green))
            .label(Span::raw(gauge_label))
            .ratio(progress);
        f.render_widget(gauge, row4[0]);

        let response_time_data: Vec<(&str, u64)> = vec![("s1", 110), ("s1", 1010)];

        let resp_histo_width = 7;

        let response_time_bar_chart = BarChart::default()
            .block(
                Block::default()
                    .title("Response Time Distribution")
                    .borders(Borders::ALL),
            )
            .data(response_time_data.as_slice())
            .bar_width(resp_histo_width as u16);

        f.render_widget(response_time_bar_chart, mid[0]);

        let mut dur_collection = durations
            .iter()
            .map(|dur| dur.as_secs_f64())
            .collect::<Vec<_>>();

        let (p99, p95, p90) = calculate_percentile(&mut dur_collection);

        let request_tuple = RequestWrapper::new(
            Number::Int(report.total_requests),
            Number::Int(report.succeeded),
            Number::Int(report.failed),
            Number::Float(report.transaction_rate),
        );

        let percentiles_floats: Vec<(&str, f64)> = vec![("p99", p99), ("p95", p95), ("p90", p90)];

        let latency_data: Vec<ListItem> = percentiles_floats
            .iter()
            .map(|&(p, value)| {
                let s = match p {
                    "p99" => Style::default().fg(Color::Cyan),
                    "p95" => Style::default().fg(Color::LightRed),
                    "p90" => Style::default().fg(Color::Green),
                    _ => Style::default(),
                };
                let header;

                header = Spans::from(vec![Span::styled(format!("{} : {:<9}", p, value), s)]);

                ListItem::new(vec![header])
            })
            .collect();

        let latency_list = List::new(latency_data)
            .block(Block::default().borders(Borders::ALL).title("Latency Data"))
            .start_corner(Corner::TopLeft);

        f.render_widget(latency_list, bottom[1]);

        let events: Vec<ListItem> = request_tuple
            .events
            .iter()
            .map(|(kpi, value)| {
                let s = match kpi {
                    &"Total Requests" => Style::default().fg(Color::Green),
                    &"Succeeded" => Style::default().fg(Color::Magenta),
                    &"Failed" => Style::default().fg(Color::Red),
                    &"Transaction Rate" => Style::default().fg(Color::Blue),
                    _ => Style::default(),
                };

                let header;

                match value {
                    Number::Int(v) => {
                        header = Spans::from(vec![Span::styled(format!("{} : {:<9}", kpi, v), s)]);
                    }
                    Number::Float(v) => {
                        header = Spans::from(vec![Span::styled(format!("{} : {:<9}", kpi, v), s)]);
                    }
                }

                ListItem::new(vec![header])
            })
            .collect();

        let events_list = List::new(events)
            .block(Block::default().borders(Borders::ALL).title("Requests Log"))
            .start_corner(Corner::TopLeft);

        f.render_widget(events_list, bottom[0]);
    })?;

    Ok(())
}

fn calculate_percentile(data: &mut Vec<f64>) -> (f64, f64, f64) {
    let (mut p99, mut p95, mut p90) = (0.0, 0.0, 0.0);

    let percentil = 100;
    // ascending sort
    data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    data.retain(|x| *x != 0.0);
    let data_len = data.len();

    // rank of highest duration
    let mut rank = (percentil as f32 / 100.0 * data_len as f32) as usize;
    // index of sorted array
    let mut max_indx = data_len as usize - 1 as usize;

    // calc percentiles and only match on the ones we are currently interested in
    while max_indx != 0 {
        let percentile = (rank as f32 / data_len as f32 * 100.0) as u8;
        rank = rank - 1;
        // println!("{}", data[max_indx - 1]);
        match percentile {
            99 => p99 = data[max_indx - 1],
            95 => p95 = data[max_indx - 1],
            90 => p90 = data[max_indx - 1],
            _ => {}
        }
        max_indx -= 1;
    }

    (p99, p95, p90)
}
