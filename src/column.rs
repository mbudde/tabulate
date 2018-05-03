use std::io::{self, Write};

#[derive(Debug)]
struct ExtraInfo {
    min_value: Option<String>,
    max_value: Option<String>,
}

#[derive(Debug)]
pub struct Column {
    samples: Vec<(usize, usize)>,
    size: Option<usize>,
    excluded: bool,
    truncated: bool,
    extra_info: Option<ExtraInfo>,
}

impl Column {
    pub fn new(initial: usize, collect_info: bool) -> Column {
        let extra = if collect_info {
            Some(ExtraInfo {
                min_value: None,
                max_value: None,
            })
        } else {
            None
        };

        Column {
            samples: vec![(initial, 1)],
            size: None,
            excluded: false,
            truncated: false,
            extra_info: extra,
        }
    }

    pub fn is_excluded(&self) -> bool {
        self.excluded
    }

    pub fn set_excluded(&mut self, is_excluded: bool) {
        self.excluded = is_excluded;
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn set_truncated(&mut self, is_truncated: bool) {
        self.truncated = is_truncated;
    }

    pub fn size(&self) -> usize {
        self.size.expect("column size has not been calculated")
    }

    pub fn calculate_size(&mut self, ratio: f64) {
        assert!(self.samples.len() > 0);

        if ratio == 0. {
            // Optimization
            self.size = Some(self.samples.iter().map(|p| p.0).max().unwrap_or(0));
        }

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
        for l in min..max + 1 {
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
        self.size = Some(best_size);
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

    pub fn print_info<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let extra = self.extra_info.take().unwrap();
        write!(out, "  Computed column size:  {}\n", self.size.unwrap())?;
        write!(out, "  Excluded:              {}\n", self.excluded)?;
        write!(out, "  Truncated:             {}\n", self.truncated)?;
        if let Some(ref min) = extra.min_value {
            write!(out, "  Min-length value:      [length {}] {:?}\n", min.len(), min)?;
        }
        if let Some(ref max) = extra.max_value {
            write!(out, "  Max-length value:      [length {}] {:?}\n", max.len(), max)?;
        }
        Ok(())
    }
}
