use crate::Report;
use std::error::Error;
use std::io;
use std::io::stdout;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{BarChart, Block, Borders, Gauge, Paragraph};
use tui::Terminal;

use crossterm::{
    style::{Print, ResetColor, SetForegroundColor},
    ExecutableCommand,
};

pub async fn write_to_t(report: &mut Report) -> Result<(), Box<dyn Error>> {
    let mut terminal = {
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)?
    };

    let start = std::time::Instant::now();

    loop {
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

            let now = std::time::Instant::now();

            let progress = ((now - start).as_secs_f64()
                / std::time::Duration::new(10, 0).as_secs_f64())
            .max(0.0)
            .min(1.0);

            // println!("{:?} {}", now, progress);

            let gauge = Gauge::default()
                .block(Block::default().title("Progress").borders(Borders::ALL))
                .gauge_style(Style::default().fg(tui::style::Color::Green))
                .label(Span::raw("Amogh"))
                .ratio(progress);
            f.render_widget(gauge, row4[0]);
        })?;
    }

    Ok(())
}
