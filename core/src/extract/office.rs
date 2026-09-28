//! Excel (xlsx, xlsm, xls, ods via calamine) and PowerPoint (pptx).
//!
//! Section headers ("— Sheet1 —", "— 3 —") are language-neutral: the UI shows
//! them as is in the preview.

use std::io::{Cursor, Read};

use calamine::{open_workbook_auto_from_rs, Data, ExcelDateTime, Reader};
use quick_xml::events::Event;

use super::SkipReason;

/// Cells beyond this are ignored (huge exports): the text stays searchable
/// without blowing up the index.
const MAX_CELLS: usize = 2_000_000;

pub fn excel(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|e| {
        let message = e.to_string().to_ascii_lowercase();
        if message.contains("password") || message.contains("encrypt") {
            SkipReason::Encrypted
        } else {
            SkipReason::Corrupt
        }
    })?;
    let mut out = String::new();
    let mut cells = 0usize;
    for name in workbook.sheet_names() {
        let Ok(range) = workbook.worksheet_range(&name) else { continue };
        out.push_str(&format!("— {name} —\n"));
        for row in range.rows() {
            cells += row.len();
            if let Some(line) = row_line(row) {
                out.push_str(&line);
                out.push('\n');
            }
            if cells > MAX_CELLS {
                return Ok(out);
            }
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_owned())
}

/// One row, cells separated by tabs. Empty cells are kept (the preview draws
/// the sheet as a table: columns must stay aligned), trailing ones dropped;
/// `None` for an empty row.
fn row_line(row: &[Data]) -> Option<String> {
    let cells: Vec<String> = row
        .iter()
        .map(|c| match c {
            Data::Empty => String::new(),
            Data::DateTime(d) => excel_date(d),
            c => c.to_string().replace(['\t', '\n', '\r'], " "),
        })
        .collect();
    let used = cells.iter().rposition(|c| !c.trim().is_empty())? + 1;
    Some(cells[..used].join("\t"))
}

/// A date cell as text (BUG-041). Excel stores dates as numbers with a date
/// format: shown as ISO 8601, the same in every language and unambiguous
/// ("2024-01-02", "2024-01-02 14:30", a time alone "14:30", a duration
/// "36:00:00"), and searchable (`2024-01-02`, `2024`).
fn excel_date(d: &ExcelDateTime) -> String {
    let value = d.as_f64();
    if d.is_duration() {
        let seconds = (value * 86_400.0).round() as i64;
        let sign = if seconds < 0 { "-" } else { "" };
        let s = seconds.abs();
        return format!("{sign}{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60);
    }
    let (year, month, day, hour, minute, second, _) = d.to_ymd_hms_milli();
    let time = if second > 0 { format!("{hour:02}:{minute:02}:{second:02}") } else { format!("{hour:02}:{minute:02}") };
    if (0.0..1.0).contains(&value) {
        time
    } else if hour == 0 && minute == 0 && second == 0 {
        format!("{year:04}-{month:02}-{day:02}")
    } else {
        format!("{year:04}-{month:02}-{day:02} {time}")
    }
}

pub fn powerpoint(bytes: &[u8]) -> Result<String, SkipReason> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| SkipReason::Corrupt)?;
    // ppt/slides/slide12.xml → 12, in presentation order.
    let mut slides: Vec<(usize, String)> = archive
        .file_names()
        .filter_map(|n| {
            let num = n.strip_prefix("ppt/slides/slide")?.strip_suffix(".xml")?.parse().ok()?;
            Some((num, n.to_owned()))
        })
        .collect();
    if slides.is_empty() {
        return Err(SkipReason::Corrupt);
    }
    slides.sort_by_key(|(n, _)| *n);

    let mut out = String::new();
    for (num, name) in slides {
        let mut xml = String::new();
        let Ok(entry) = archive.by_name(&name) else { continue };
        if entry.take(64 * 1024 * 1024).read_to_string(&mut xml).is_err() {
            continue;
        }
        out.push_str(&format!("— {num} —\n"));
        out.push_str(&drawing_text(&xml));
        out.push_str("\n\n");
    }
    Ok(out.trim_end().to_owned())
}

/// Text of DrawingML (`a:t` runs, one line per `a:p` paragraph).
fn drawing_text(xml: &str) -> String {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut out = String::new();
    let mut in_text = false;
    let mut line = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == "t" => in_text = true,
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                "t" => in_text = false,
                "p" => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        out.push_str(trimmed);
                        out.push('\n');
                    }
                    line.clear();
                }
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => line.push_str(&t.html_content()),
            Ok(Event::GeneralRef(r)) if in_text => line.push(match r.resolve_char_ref() {
                Ok(Some(c)) => c,
                _ => match &*r {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => ' ',
                },
            }),
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_keep_their_columns() {
        let row = [Data::String("A".into()), Data::Empty, Data::Float(3.5), Data::Empty, Data::Empty];
        assert_eq!(row_line(&row).as_deref(), Some("A\t\t3.5"));
        assert_eq!(row_line(&[Data::Empty, Data::String("x\ty".into())]).as_deref(), Some("\tx y"));
        assert_eq!(row_line(&[Data::Empty, Data::Empty]), None);
    }

    #[test]
    fn dates_read_as_dates() {
        use calamine::ExcelDateTimeType::{DateTime, TimeDelta};
        let date = |v: f64| Data::DateTime(ExcelDateTime::new(v, DateTime, false));
        let row = [date(45293.0), date(45293.75), date(0.5), Data::Float(137.5)];
        assert_eq!(row_line(&row).as_deref(), Some("2024-01-02\t2024-01-02 18:00\t12:00\t137.5"));
        assert_eq!(excel_date(&ExcelDateTime::new(45293.0 + 1.0 / 86_400.0 * 5.0, DateTime, false)), "2024-01-02 00:00:05");
        assert_eq!(excel_date(&ExcelDateTime::new(1.5, TimeDelta, false)), "36:00:00");
        // Workbooks from old Macs count from 1904.
        assert_eq!(excel_date(&ExcelDateTime::new(43831.0, DateTime, true)), "2024-01-02");
    }
}
