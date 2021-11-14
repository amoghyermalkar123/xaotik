use crate::Report;
use crossterm::style::Stylize;
use std::error::Error;
use std::io::{self, Stdout};
use std::io::stdout;
use std::sync::Arc;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Corner, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{
    BarChart, Block, Borders, Chart, Gauge, List, ListItem, ListState, Paragraph, Sparkline, Table,
};
use tui::Terminal;

use crossterm::{
    style::{Print, ResetColor, SetForegroundColor},
    ExecutableCommand,
};

use tokio::sync::mpsc::{self, Receiver};

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
) -> Result<(), Box<dyn Error>> {
    let mut terminal = {
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)?
    };

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
                // let progress = (report.total_requests * 100) / 100;
                // println!("{} {}",report.total_requests,  progress);
                draw(&mut terminal, report)?;
            }
            None => {
                terminal.clear()?;
                break;
            }
        }
    }
    Ok(())
}

fn draw(terminal: &mut Terminal<CrosstermBackend<Stdout>>, report : &Report) -> Result<(), Box<dyn Error>> {
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

        let progress = (report.total_requests * 100) / 100;

        let mid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
            )
            .split(row4[1]);

        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
            )
            .split(row4[2]);

        let gauge_label = format!(
            "{} / 100%",
            progress, 
        );

        let gauge = Gauge::default()
            .block(Block::default().title("Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(tui::style::Color::Green))
            .label(Span::raw(gauge_label))
            .percent(progress as u16);
        f.render_widget(gauge, row4[0]);

        let response_time_data: Vec<(&str, u64)> = vec![("s1", 110), ("s1", 1010)];

        let resp_histo_width = 7;

        let latency_bar_chart = BarChart::default()
            .block(
                Block::default()
                    .title("Latency Distribution histogram")
                    .borders(Borders::ALL),
            )
            .data(response_time_data.as_slice())
            .bar_width(resp_histo_width as u16);
        f.render_widget(latency_bar_chart, mid[0]);

        let data: Vec<(&str, u64)> = vec![("p99", 1010), ("p95", 1010)];

        let resp_histo_width = 7;

        let latency_bar_chart = BarChart::default()
            .block(
                Block::default()
                    .title("Latency Distribution histogram")
                    .borders(Borders::ALL),
            )
            .data(data.as_slice())
            .bar_width(resp_histo_width as u16);
        f.render_widget(latency_bar_chart, mid[1]);

        let request_tuple = RequestWrapper::new(
            Number::Int(report.total_requests),
            Number::Int(report.succeeded),
            Number::Int(report.failed),
            Number::Float(report.transaction_rate),
        );

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
                        header = Spans::from(vec![Span::styled(
                            format!("{} : {:<9}", kpi, v),
                            s,
                        )]);
                    }
                    Number::Float(v) => {
                        header = Spans::from(vec![Span::styled(
                            format!("{} : {:<9}", kpi, v),
                            s,
                        )]);
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