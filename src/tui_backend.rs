use crate::MachineDetails;
use crate::Report;

use crossterm::style::Stylize;
use crossterm::{
    style::{Print, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use netlink_wi::NlSocket;
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
use tui::style::{Color, Modifier, Style};
use tui::symbols;
use tui::text::{Span, Spans};
use tui::widgets::{
    Axis, BarChart, Block, Borders, Chart, Dataset, Gauge, GraphType, List, ListItem, ListState,
    Paragraph, Sparkline, Table,
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
    let mut terminal = {
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)?
    };

    let mut durations: Vec<std::time::Duration> = Vec::new();
    terminal.clear()?;

    let mut p99_data: Vec<(f64, f64)> = Vec::new();

    loop {
        match report_receiver.recv().await {
            Some(received_report) => {
                report.add_report(
                    received_report.succeeded,
                    received_report.failed,
                    received_report.total_requests,
                    received_report.elapsed,
                );

                durations.push(received_report.duration);

                report.transaction_rate =
                    test_started_at.elapsed().as_secs_f64() / received_report.total_requests as f64;
                let mut machine_details: MachineDetails = MachineDetails::new();

                let socket = NlSocket::connect().unwrap();
                let interfaces = socket.list_interfaces().unwrap();
                for interface in interfaces {
                    let interface = interface.unwrap();
                    let stations = socket.list_stations(interface.interface_index).unwrap();
                    for station in stations {
                        let station = station.unwrap();
                        // station.tx_bitrate.unwrap_or(0);
                        machine_details.avg_signal = match station.average_signal {
                            Some(v) => v,
                            None => 0,
                        };
                        machine_details.rx_bitrate = match station.rx_bitrate {
                            Some(v) => v.bitrate as f32 * 100.0 / 1000 as f32,
                            None => 0.0,
                        };
                        machine_details.tx_bitrate = match station.tx_bitrate {
                            Some(v) => v.bitrate as f32 * 100.0 / 1000 as f32,
                            None => 0.0,
                        };
                        machine_details.frequency = match interface.frequency {
                            Some(v) => v,
                            None => 0,
                        };
                        machine_details.ssid = match interface.ssid {
                            Some(ref v) => v.to_string(),
                            None => 0.to_string(),
                        };
                    }
                }

                let mut dur_collection = durations
                    .iter()
                    .map(|dur| dur.as_secs_f64())
                    .collect::<Vec<_>>();

                let (p99, p95, p90) = calculate_percentile(&mut dur_collection);

                p99_data.append(&mut vec![(report.elapsed as f64, p99)]);

                let p99data = p99_data.clone();

                let x_elapsed = (test_started_at.elapsed().as_secs_f64()).trunc();
                let y_offset = (p99 + 0.9).trunc();

                draw(
                    &mut terminal,
                    report,
                    test_started_at,
                    total_duration_for_test,
                    machine_details,
                    p99,
                    p95,
                    p90,
                    p99data,
                    x_elapsed,
                    y_offset,
                )?;
            }
            None => {
                // std::thread::sleep(std::time::Duration::from_secs(12));
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
    start: Instant,
    total_test_time: Duration,
    machine_details: MachineDetails,
    p99: f64,
    p95: f64,
    p90: f64,
    p99_data: Vec<(f64, f64)>,
    x_elapsed: f64,
    y_axis_offset: f64,
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

        let err_code_data: Vec<(&str, u64)> = vec![("s1", 110), ("s1", 1010)];

        let err_histo_width = 7;

        let err_code_bar_chart = BarChart::default()
            .block(
                Block::default()
                    .title("Error Code Distribution")
                    .borders(Borders::ALL),
            )
            .data(err_code_data.as_slice())
            .bar_width(err_histo_width as u16);

        f.render_widget(err_code_bar_chart, mid[0]);

        let ssid = Spans::from(vec![Span::styled(
            format!("{} : {:<9}", "SSID", machine_details.ssid),
            Style::default().fg(Color::Cyan),
        )]);

        let frequency = Spans::from(vec![Span::styled(
            format!("{} : {} MHz ", "Frequency", machine_details.frequency),
            Style::default().fg(Color::Cyan),
        )]);

        let tx_bitrate = Spans::from(vec![Span::styled(
            format!(
                "{} : {} Mb/s",
                "Transmission Bitrate", machine_details.tx_bitrate
            ),
            Style::default().fg(Color::Cyan),
        )]);

        let rx_bitrate = Spans::from(vec![Span::styled(
            format!(
                "{} : {} Mb/s",
                "Receive Bitrate", machine_details.rx_bitrate
            ),
            Style::default().fg(Color::Cyan),
        )]);

        let avg_signal = Spans::from(vec![Span::styled(
            format!(
                "{} : {} dBm",
                "Avegrage Signal Strength", machine_details.avg_signal
            ),
            Style::default().fg(Color::Cyan),
        )]);

        let details: Vec<ListItem> = vec![
            ListItem::new(vec![ssid]),
            ListItem::new(vec![frequency]),
            ListItem::new(vec![tx_bitrate]),
            ListItem::new(vec![rx_bitrate]),
            ListItem::new(vec![avg_signal]),
        ];

        let machine_details_list = List::new(details)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Machine Details"),
            )
            .start_corner(Corner::TopLeft);

        f.render_widget(machine_details_list, mid[1]);

        let bottomest = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(row4[3]);

        let datasets = vec![Dataset::default()
            .name("data")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&p99_data)];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        "Chart 3",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("time (sec)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, x_elapsed])
                    .labels(vec![
                        Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                        // Span::raw("5"),
                        Span::styled(
                            x_elapsed.to_string(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("p99 latency")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, y_axis_offset])
                    .labels(vec![
                        Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                        // Span::raw("2.5"),
                        Span::styled(
                            y_axis_offset.to_string(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            );

        f.render_widget(chart, bottomest[0]);

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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Request Details"),
            )
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
