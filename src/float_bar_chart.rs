use ordered_float::OrderedFloat;
use std::cmp::min;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    symbols,
    widgets::{Block, Widget},
};
use unicode_width::UnicodeWidthStr;

// ============================================CURRENTLY UN-USED================================================

#[derive(Debug, Clone)]
pub struct BarChart<'a> {
    /// Block to wrap the widget in
    block: Option<Block<'a>>,
    /// The width of each bar
    bar_width: u16,
    /// The gap between each bar
    bar_gap: u16,
    /// Set of symbols used to display the data
    bar_set: symbols::bar::Set,
    /// Style of the bars
    bar_style: Style,
    /// Style of the values printed at the bottom of each bar
    value_style: Style,
    /// Style of the labels printed under each bar
    label_style: Style,
    /// Style for the widget
    style: Style,
    /// Slice of (label, value) pair to plot on the chart
    data: &'a [(&'a str, f64)],
    /// Value necessary for a bar to reach the maximum height (if no value is specified,
    /// the maximum value in the data is taken as reference)
    max: Option<f64>,
    /// Values to display on the bar (computed when the data is passed to the widget)
    values: Vec<String>,
}

impl<'a> Default for BarChart<'a> {
    fn default() -> BarChart<'a> {
        BarChart {
            block: None,
            max: None,
            data: &[],
            values: Vec::new(),
            bar_style: Style::default(),
            bar_width: 1,
            bar_gap: 1,
            bar_set: symbols::bar::NINE_LEVELS,
            value_style: Default::default(),
            label_style: Default::default(),
            style: Default::default(),
        }
    }
}

impl<'a> BarChart<'a> {
    pub fn data(mut self, data: &'a [(&'a str, f64)]) -> BarChart<'a> {
        self.data = data;
        self.values = Vec::with_capacity(self.data.len());
        for &(_, v) in self.data {
            self.values.push(format!("{}", v));
        }
        self
    }

    pub fn block(mut self, block: Block<'a>) -> BarChart<'a> {
        self.block = Some(block);
        self
    }

    pub fn max(mut self, max: f64) -> BarChart<'a> {
        self.max = Some(max);
        self
    }

    pub fn bar_style(mut self, style: Style) -> BarChart<'a> {
        self.bar_style = style;
        self
    }

    pub fn bar_width(mut self, width: u16) -> BarChart<'a> {
        self.bar_width = width;
        self
    }

    pub fn bar_gap(mut self, gap: u16) -> BarChart<'a> {
        self.bar_gap = gap;
        self
    }

    pub fn bar_set(mut self, bar_set: symbols::bar::Set) -> BarChart<'a> {
        self.bar_set = bar_set;
        self
    }

    pub fn value_style(mut self, style: Style) -> BarChart<'a> {
        self.value_style = style;
        self
    }

    pub fn label_style(mut self, style: Style) -> BarChart<'a> {
        self.label_style = style;
        self
    }

    pub fn style(mut self, style: Style) -> BarChart<'a> {
        self.style = style;
        self
    }
}

impl<'a> Widget for BarChart<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);

        let chart_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if chart_area.height < 2 {
            return;
        }

        let max = match self.max {
            Some(m) => m,
            None => {
                let ord_float = self
                    .data
                    .iter()
                    .map(|t| OrderedFloat(t.1))
                    .max()
                    .unwrap_or_default();
                ord_float.into()
            }
        };

        let max_index = min(
            (chart_area.width / (self.bar_width + self.bar_gap)) as usize,
            self.data.len(),
        );

        let mut data = self
            .data
            .iter()
            .take(max_index)
            .map(|&(l, v)| {
                let ordf = OrderedFloat(v);
                let maximum = ordf.max(OrderedFloat(max));
                let maximum : f64 = maximum.into(); 
                (
                    l,
                    v * f64::from(chart_area.height - 1) * 8.0 / maximum,
                )
            })
            .collect::<Vec<(&str, f64)>>();

        // for j in (0..chart_area.height - 1).rev() {
        //     for (i, d) in data.iter_mut().enumerate() {
        //         let symbol = match d.1 {
        //             0.0 => self.bar_set.empty,
        //             1.0 => self.bar_set.one_eighth,
        //             2.0 => self.bar_set.one_quarter,
        //             3.0 => self.bar_set.three_eighths,
        //             4.0 => self.bar_set.half,
        //             5.0 => self.bar_set.five_eighths,
        //             6.0 => self.bar_set.three_quarters,
        //             7.0 => self.bar_set.seven_eighths,
        //             _ => self.bar_set.full,
        //         };

        //         for x in 0..self.bar_width {
        //             buf.get_mut(
        //                 chart_area.left() + i as u16 * (self.bar_width + self.bar_gap) + x,
        //                 chart_area.top() + j,
        //             )
        //             .set_symbol(symbol)
        //             .set_style(self.bar_style);
        //         }

        //         if d.1 > 8.0 {
        //             d.1 -= 8.0;
        //         } else {
        //             d.1 = 0.0;
        //         }
        //     }
        // }

        for (i, &(label, value)) in self.data.iter().take(max_index).enumerate() {
            if value != 0.0 {
                let value_label = &self.values[i];
                let width = value_label.width() as u16;
                if width < self.bar_width {
                    buf.set_string(
                        chart_area.left()
                            + i as u16 * (self.bar_width + self.bar_gap)
                            + (self.bar_width - width) / 2,
                        chart_area.bottom() - 2,
                        value_label,
                        self.value_style,
                    );
                }
            }
            buf.set_stringn(
                chart_area.left() + i as u16 * (self.bar_width + self.bar_gap),
                chart_area.bottom() - 1,
                label,
                self.bar_width as usize,
                self.label_style,
            );
        }
    }
}