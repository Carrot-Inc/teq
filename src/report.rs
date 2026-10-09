//! What `--time` prints: sections of rows, each row a label with a count, a duration and a
//! note, the columns aligned over the whole report.

use std::time::Duration;

#[derive(Default)]
pub struct Report {
    sections: Vec<Section>,
}

pub struct Section {
    title: &'static str,
    rows: Vec<Row>,
    detail: Vec<String>,
}

#[derive(Default)]
pub struct Row {
    label: String,
    count: Option<String>,
    time: Option<String>,
    note: String,
}

impl Report {
    pub fn section(&mut self, title: &'static str) -> &mut Section {
        self.sections.push(Section { title, rows: Vec::new(), detail: Vec::new() });
        self.sections.last_mut().unwrap()
    }

    /// The phases of a command over `lines` source lines, in their order, the total last.
    pub fn phases(&mut self, lines: usize, phases: &[(&str, Duration, &str)], total: Duration) {
        let section = self.section("phases");
        section.row("lines").count(lines);
        for (name, time, note) in phases {
            section.row(name).time(*time).note(note.to_string());
        }
        section.row("total").time(total);
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub fn render(&self) -> String {
        let rows = || self.sections.iter().flat_map(|s| s.rows.iter());
        let label = rows().map(|r| r.label.chars().count()).max().unwrap_or(0);
        let count = rows().filter_map(|r| r.count.as_ref()).map(|c| c.len()).max().unwrap_or(0);
        let time = rows().filter_map(|r| r.time.as_ref()).map(|t| t.len()).max().unwrap_or(0);
        let mut out = String::new();
        for section in &self.sections {
            out.push_str(section.title);
            out.push('\n');
            for row in &section.rows {
                let mut line = format!("  {:<label$}", row.label);
                if count > 0 {
                    line.push_str(&format!("  {:>count$}", row.count.as_deref().unwrap_or("")));
                }
                if time > 0 {
                    line.push_str(&format!("  {:>time$}", row.time.as_deref().unwrap_or("")));
                }
                if !row.note.is_empty() {
                    line.push_str("  ");
                    line.push_str(&row.note);
                }
                out.push_str(line.trim_end());
                out.push('\n');
            }
            for line in &section.detail {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out.truncate(out.trim_end().len());
        out
    }
}

impl Section {
    pub fn row(&mut self, label: &str) -> &mut Row {
        self.rows.push(Row { label: label.to_string(), ..Row::default() });
        self.rows.last_mut().unwrap()
    }

    pub fn detail(&mut self, line: String) {
        self.detail.push(line);
    }
}

impl Row {
    pub fn count(&mut self, n: usize) -> &mut Row {
        self.count = Some(grouped(n as u64));
        self
    }

    pub fn time(&mut self, d: Duration) -> &mut Row {
        self.time = Some(format!("{:.2} ms", d.as_secs_f64() * 1000.0));
        self
    }

    pub fn note(&mut self, note: String) -> &mut Row {
        self.note = note;
        self
    }
}

/// `22,199,683`.
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `21.2 MB`, `58.0 KB`, `512 bytes`.
pub fn bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let n = n as f64;
    if n >= KB * KB {
        format!("{:.1} MB", n / (KB * KB))
    } else if n >= KB {
        format!("{:.1} KB", n / KB)
    } else {
        format!("{} bytes", n)
    }
}
