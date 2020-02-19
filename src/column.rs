use std::io::{self, Write};

#[derive(Debug, Clone)]
struct Options {
    excluded: bool,
    truncated: bool,
}

#[derive(Debug, Clone)]
struct ExtraInfo {
    min_value: Option<String>,
    max_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MeasureColumn {
    samples: Vec<(usize, usize)>,
    opts: Options,
    extra_info: Option<ExtraInfo>,
}

#[derive(Debug)]
pub struct Column {
    size: usize,
    opts: Options,
    extra_info: Option<ExtraInfo>,
}

impl MeasureColumn {
    pub fn new(collect_info: bool) -> MeasureColumn {
        let extra = if collect_info {
            Some(ExtraInfo {
                min_value: None,
                max_value: None,
            })
        } else {
            None
        };

        MeasureColumn {
            samples: vec![],
            opts: Options {
                excluded: false,
                truncated: false,
            },
            extra_info: extra,
        }
    }

    pub fn set_excluded(&mut self, is_excluded: bool) {
        self.opts.excluded = is_excluded;
    }

    pub fn set_truncated(&mut self, is_truncated: bool) {
        self.opts.truncated = is_truncated;
    }

    pub fn add_sample(&mut self, sample: &str) {
        let size = sample.len();
        match self.samples.binary_search_by_key(&size, |t| t.0) {
            Ok(i) => self.samples[i].1 += 1,
            Err(i) => self.samples.insert(i, (size, 1)),
        }
        if let Some(ref mut extra) = self.extra_info {
            if extra.min_value.as_ref().map(|s| size < s.len()).unwrap_or(true) {
                extra.min_value = Some(sample.to_string());
            }
            if extra.max_value.as_ref().map(|s| size > s.len()).unwrap_or(true) {
                extra.max_value = Some(sample.to_string());
            }
        }
    }

    pub fn calculate_size(&self, ratio: f64) -> Column {
        assert!(!self.samples.is_empty());

        let best_size = if ratio == 0. {
            // Optimization
            self.samples.iter().map(|p| p.0).max().unwrap_or(0)
        } else {
            let n: usize = self.samples.iter().map(|p| p.1).sum();
            let min = self.samples.iter().map(|p| p.0).min().unwrap();
            let max = self.samples.iter().map(|p| p.0).max().unwrap();
            let spread = (0.7 + 20.0 / (1 + max - min) as f64).powi(2);
            let prob = self.samples
                .iter()
                .map(|&(s, x)| (s, x as f64 / n as f64))
                .collect::<Vec<_>>();

            let mut best_score = ::std::f64::INFINITY;
            let mut best_size = max;
            for l in min..=max {
                let waste: f64 = prob.iter()
                    .take_while(|&&(s, _)| s < l)
                    .map(|&(s, p)| p * l.saturating_sub(s) as f64)
                    .sum();
                let overflow: f64 = prob.iter()
                    .skip_while(|&&(s, _)| s <= l)
                    .map(|&(s, p)| p * s.saturating_sub(l) as f64)
                    .sum();

                let score = ratio * (1.0 + waste) + (1.0 + overflow).powi(2) * spread;

                if score < best_score {
                    best_score = score;
                    best_size = l;
                } else {
                    break;
                }
            }

            best_size
        };

        Column {
            size: best_size,
            opts: self.opts.clone(),
            extra_info: self.extra_info.clone(),
        }
    }

}

impl Column {
    pub fn is_excluded(&self) -> bool {
        self.opts.excluded
    }

    pub fn print_cell<W: Write>(&self, out: &mut W, cell: &str, overflow: usize, last: bool) -> io::Result<usize> {
        if last {
            write!(out, "{}", cell)?;
            Ok(0)
        } else {
            let out_width = self.size.saturating_sub(overflow);
            if self.opts.truncated && cell.len() > out_width {
                if out_width > 0 {
                    write!(out, "{}…", &cell[0..out_width - 1])?;
                    Ok(0)
                } else {
                    write!(out, "…")?;
                    Ok(1)
                }
            } else {
                write!(out, "{:1$}", cell, out_width)?;
                if cell.len() < self.size {
                    Ok(overflow.saturating_sub(self.size.saturating_sub(cell.len())))
                } else {
                    Ok(overflow + cell.len().saturating_sub(self.size))
                }
            }
        }
    }

    pub fn print_info<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let extra = self.extra_info.take().unwrap();
        writeln!(out, "  Computed column size:  {}", self.size)?;
        writeln!(out, "  Excluded:              {}", self.opts.excluded)?;
        writeln!(out, "  Truncated:             {}", self.opts.truncated)?;
        if let Some(ref min) = extra.min_value {
            writeln!(out, "  Min-length value:      [length {}] {:?}", min.len(), min)?;
        }
        if let Some(ref max) = extra.max_value {
            writeln!(out, "  Max-length value:      [length {}] {:?}", max.len(), max)?;
        }
        Ok(())
    }
}
