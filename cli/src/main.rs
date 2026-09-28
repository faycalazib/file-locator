//! prospector-cli (lot 7.1): Prospector from the command line, on the same
//! indexes as the app (`prospector_core::locate`).
//!
//! Exit codes: 0 = found something, 1 = nothing found, 2 = invalid command
//! or query, 3 = data unavailable (unknown site, index busy, app open…).

mod output;

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use clap::{Args, Parser, Subcommand, ValueEnum};
use prospector_core::locate::{self, Location};
use prospector_core::share::{self, Lease, Role};
use prospector_core::search::DateField;
use prospector_core::{CoreError, Engine, ScanTarget, SearchRequest, SiteRecord};

use output::{Format, Printer};

#[derive(Parser)]
#[command(
    name = "prospector-cli",
    version,
    about = "Full-text search in your files, from the command line (Prospector).",
    after_help = "Exit codes: 0 found, 1 nothing found, 2 invalid command or query, 3 data unavailable.\n\
                  Query syntax: words, \"exact phrase\", AND / OR / NOT, -word, a NEAR b, NEAR:20, LIKE word, LINES:3-5, /regex/."
)]
struct Cli {
    /// Data folder (catalog and indexes); by default the app's.
    #[arg(long, global = true, value_name = "FOLDER")]
    data: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search the indexed sites (instant).
    Search {
        /// What to look for; "" with --name lists files by name only.
        query: String,
        /// Site name or id (repeat for several); all sites by default.
        #[arg(long = "site", value_name = "SITE")]
        sites: Vec<String>,
        /// Site group made in the app (repeat for several).
        #[arg(long = "group", value_name = "GROUP")]
        groups: Vec<String>,
        /// At most this many files.
        #[arg(long, default_value_t = 200)]
        limit: usize,
        #[command(flatten)]
        criteria: Criteria,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Read folders directly, without an index; results are printed as they are found.
    Scan {
        /// What to look for; "" with --name lists files by name only.
        query: String,
        /// Folders to read.
        #[arg(required = true, value_name = "FOLDER")]
        paths: Vec<PathBuf>,
        #[command(flatten)]
        criteria: Criteria,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Bring sites up to date (only files added, changed or removed are read).
    Index {
        /// Site names or ids.
        #[arg(value_name = "SITE", required_unless_present_any = ["all", "groups"])]
        sites: Vec<String>,
        /// Site group made in the app (repeat for several).
        #[arg(long = "group", value_name = "GROUP")]
        groups: Vec<String>,
        /// Every site.
        #[arg(long)]
        all: bool,
    },
    /// List the dig sites.
    Sites {
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Run by the uninstaller: removes the program's folder from the user's PATH.
    #[command(hide = true)]
    UninstallCleanup,
}

#[derive(Clone, Copy, ValueEnum)]
enum DateOn {
    Modified,
    Created,
    Accessed,
}

#[derive(Clone, Copy, ValueEnum)]
enum Attribute {
    ReadOnly,
    Hidden,
    System,
}

/// The criteria of the app's search bar and filters.
#[derive(Args)]
struct Criteria {
    /// File names: `*.pdf; invoice-*`, `!draft*` to exclude, or `/regex/`.
    #[arg(long, value_name = "PATTERN")]
    name: Option<String>,
    /// Kinds: pdf, word, excel, powerpoint, text, code, archive, email, image.
    #[arg(long, value_delimiter = ',', value_name = "KIND")]
    kind: Vec<String>,
    /// Document languages: en, fr, es, ar.
    #[arg(long, value_delimiter = ',', value_name = "LANG")]
    lang: Vec<String>,
    /// From this day (YYYY-MM-DD).
    #[arg(long, value_name = "DATE")]
    since: Option<String>,
    /// Up to this day included (YYYY-MM-DD).
    #[arg(long, value_name = "DATE")]
    until: Option<String>,
    /// The date --since / --until apply to.
    #[arg(long, value_enum, default_value_t = DateOn::Modified)]
    date: DateOn,
    /// At least this size (500, 10KB, 2MB, 1GB).
    #[arg(long, value_name = "SIZE", value_parser = output::parse_size)]
    min_size: Option<u64>,
    /// At most this size.
    #[arg(long, value_name = "SIZE", value_parser = output::parse_size)]
    max_size: Option<u64>,
    /// Required attributes (read on the disk).
    #[arg(long = "attr", value_enum, value_delimiter = ',')]
    attributes: Vec<Attribute>,
    /// MD5 or SHA-256 of the file.
    #[arg(long, value_name = "DIGEST")]
    hash: Option<String>,
    /// Data to detect: iban, bic, rib, card, email, phone, ip, amount, vat, siren, nir, dni, mrz.
    #[arg(long, value_delimiter = ',', value_name = "DETECTOR")]
    detect: Vec<String>,
    /// The whole query is one regular expression.
    #[arg(long)]
    regex: bool,
    /// Exact case.
    #[arg(long)]
    case: bool,
    /// Also inside longer words (not whole words only).
    #[arg(long)]
    partial: bool,
    /// Tolerate typos (approximate matches are marked).
    #[arg(long)]
    fuzzy: bool,
    /// Look for folders (by name) instead of files.
    #[arg(long)]
    folders: bool,
    /// Terms read from a file, one per line (first column of a CSV): files with at least one of them.
    #[arg(long, value_name = "FILE")]
    terms_file: Option<PathBuf>,
    /// With --terms-file: files with all of its terms.
    #[arg(long, requires = "terms_file")]
    terms_all: bool,
}

/// A failure, with its exit code.
struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    fn usage(message: impl Into<String>) -> Self {
        Self { code: 2, message: message.into() }
    }

    fn data(message: impl Into<String>) -> Self {
        Self { code: 3, message: message.into() }
    }
}

impl From<CoreError> for Failure {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::InvalidQuery { .. } | CoreError::InvalidHash { .. } => Failure::usage(e.to_string()),
            _ => Failure::data(e.to_string()),
        }
    }
}

impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        Failure::data(e.to_string())
    }
}

impl Criteria {
    fn request(&self, query: String) -> Result<SearchRequest, Failure> {
        for kind in &self.kind {
            if prospector_core::kind::FileKind::from_code(kind).is_none() {
                return Err(Failure::usage(format!("unknown kind: {kind}")));
            }
        }
        for lang in &self.lang {
            if !matches!(lang.as_str(), "en" | "fr" | "es" | "ar") {
                return Err(Failure::usage(format!("unknown language: {lang} (en, fr, es, ar)")));
            }
        }
        let detectors = self
            .detect
            .iter()
            .map(|d| serde_json::from_value(serde_json::Value::String(d.to_ascii_lowercase())).map_err(|_| Failure::usage(format!("unknown detector: {d}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let day = |text: &Option<String>| -> Result<Option<u64>, Failure> {
            text.as_deref()
                .map(|t| output::parse_date(t).ok_or_else(|| Failure::usage(format!("not a date (YYYY-MM-DD): {t}"))))
                .transpose()
        };
        let term_list = match &self.terms_file {
            Some(path) => prospector_core::terms::read_list(path)?.terms,
            None => Vec::new(),
        };
        if self.terms_file.is_some() && term_list.is_empty() {
            return Err(Failure::usage("the terms file holds no term"));
        }
        Ok(SearchRequest {
            query,
            term_list,
            term_list_all: self.terms_all,
            fuzzy: self.fuzzy,
            case_sensitive: self.case,
            whole_word: !self.partial,
            regex: self.regex,
            kinds: self.kind.clone(),
            langs: self.lang.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            date_field: match self.date {
                DateOn::Modified => DateField::Modified,
                DateOn::Created => DateField::Created,
                DateOn::Accessed => DateField::Accessed,
            },
            date_from: day(&self.since)?,
            // The last day is included.
            date_to: day(&self.until)?.map(|t| t + 86_400),
            attributes: self
                .attributes
                .iter()
                .map(|a| match a {
                    Attribute::ReadOnly => "readOnly",
                    Attribute::Hidden => "hidden",
                    Attribute::System => "system",
                })
                .map(str::to_owned)
                .collect(),
            hash: self.hash.clone().unwrap_or_default(),
            name_pattern: self.name.clone().unwrap_or_default(),
            folders: self.folders,
            detectors,
            ..Default::default()
        })
    }
}

/// The sites named on the command line (by id or name, case ignored), and
/// those of the groups named (lot 7.2); all of them when none is named.
fn pick_sites(engine: &Engine, names: &[String], groups: &[String]) -> Result<Vec<SiteRecord>, Failure> {
    let all = engine.sites();
    if names.is_empty() && groups.is_empty() {
        return Ok(all);
    }
    let same = |a: &str, b: &str| a == b || a.to_lowercase() == b.to_lowercase();
    let mut ids: Vec<String> = Vec::new();
    for name in names {
        let site = all.iter().find(|s| same(&s.id, name) || same(&s.name, name));
        let site = site.ok_or_else(|| Failure::data(format!("unknown site: {name} (see `prospector-cli sites`)")))?;
        ids.push(site.id.clone());
    }
    let known = engine.site_groups();
    for name in groups {
        let group = known.iter().find(|g| same(&g.id, name) || same(&g.name, name));
        let group = group.ok_or_else(|| Failure::data(format!("unknown group: {name} (see `prospector-cli sites`)")))?;
        ids.extend(group.site_ids.iter().cloned());
    }
    Ok(all.into_iter().filter(|s| ids.contains(&s.id)).collect())
}

/// Whether the Prospector app is open (the mutex of its single instance).
#[cfg(windows)]
fn app_is_open(portable: bool) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::OpenMutexW;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    let name: Vec<u16> = format!("{}-sim", locate::identifier(portable)).encode_utf16().chain([0]).collect();
    // SAFETY: a NUL-terminated name; the handle, if any, is closed at once.
    unsafe {
        let handle = OpenMutexW(SYNCHRONIZE, 0, name.as_ptr());
        if handle.is_null() {
            return false;
        }
        CloseHandle(handle);
    }
    true
}

#[cfg(not(windows))]
fn app_is_open(_portable: bool) -> bool {
    false
}

fn found(count: usize) -> ExitCode {
    ExitCode::from(if count > 0 { 0 } else { 1 })
}

fn run(cli: Cli) -> Result<ExitCode, Failure> {
    if let Command::UninstallCleanup = cli.command {
        let exe = std::env::current_exe()?;
        if let Some(dir) = exe.parent() {
            prospector_core::userpath::set(dir, false)?;
        }
        return Ok(ExitCode::SUCCESS);
    }
    let location = Location::new(locate::current_portable_root()).ok_or_else(|| Failure::data("cannot find the user's data folder"))?;
    let (default_data, ocr) = location.read_settings();
    let data_dir = cli.data.clone().unwrap_or(default_data);
    // `--data`: everything in that folder; otherwise this user's saved
    // searches and groups (lot 7.3).
    let personal_dir = if cli.data.is_some() { data_dir.clone() } else { location.personal_dir() };
    // Shared index (lot 7.3): read only while another PC keeps it up to date.
    let open = || -> Result<Engine, Failure> {
        let engine = Engine::open_with(&data_dir, &personal_dir)?;
        engine.set_reader_of(Lease::new(&data_dir).holder(share::now()));
        Ok(engine)
    };
    prospector_core::extract::ocr::set_enabled(ocr);
    let stdout = std::io::stdout();
    let quiet = !std::io::stderr().is_terminal();

    match cli.command {
        Command::Sites { format } => {
            let engine = open()?;
            let sites = engine.sites();
            output::sites(format, &sites, &engine.site_groups(), stdout)?;
            Ok(found(sites.len()))
        }
        Command::Search { query, sites, groups, limit, criteria, format } => {
            let engine = open()?;
            let sites = pick_sites(&engine, &sites, &groups)?;
            let mut request = criteria.request(query)?;
            request.limit = Some(limit);
            let ids: Vec<String> = sites.iter().map(|s| s.id.clone()).collect();
            let response = engine.search(&ids, &request)?;
            let name_of = |id: &str| sites.iter().find(|s| s.id == id).map_or(id, |s| s.name.as_str()).to_owned();
            let mut printer = Printer::new(format, stdout);
            for hit in &response.hits {
                printer.hit(hit, &name_of(&hit.site_id))?;
            }
            printer.finish(serde_json::json!({ "totalFiles": response.total_files, "tookMs": response.took_ms }))?;
            if format == Format::Text && !quiet {
                eprintln!("{} of {} files, {} ms", response.hits.len(), response.total_files, response.took_ms);
            }
            Ok(found(response.hits.len()))
        }
        Command::Scan { query, paths, criteria, format } => {
            for folder in &paths {
                if !folder.is_dir() {
                    return Err(Failure::data(format!("folder not found: {}", folder.display())));
                }
            }
            let request = criteria.request(query)?;
            let target = ScanTarget { site_id: "scan".to_owned(), roots: paths };
            let printer = Mutex::new(Printer::new(format, stdout));
            let summary = prospector_core::live_scan(
                &[target],
                &[],
                &request,
                &AtomicBool::new(false),
                &|hit| {
                    if let Ok(mut p) = printer.lock() {
                        // A closed pipe (`| head`) is not an error worth reporting.
                        let _ = p.hit(&hit, "");
                    }
                },
                &|_, _| {},
            )?;
            let printer = printer.into_inner().map_err(|_| Failure::data("output failed"))?;
            printer.finish(serde_json::json!({ "filesScanned": summary.files_scanned, "tookMs": summary.took_ms }))?;
            if format == Format::Text && !quiet {
                eprintln!("{} files found, {} read, {} ms", summary.hits, summary.files_scanned, summary.took_ms);
            }
            Ok(found(summary.hits))
        }
        Command::Index { sites, groups, all } => {
            // The app keeps its sites up to date itself; two writers on the
            // same index and catalog would step on each other.
            if cli.data.is_none() && app_is_open(location.portable.is_some()) {
                return Err(Failure::data("Prospector is open and already keeps its sites up to date; close it to index from here."));
            }
            // Indexing takes the lease of the data folder, and gives it back.
            let mut lease = Lease::new(&data_dir);
            if let Role::Reader { pc } = lease.claim(share::now()) {
                return Err(Failure::data(format!("{pc} keeps this shared index up to date; index from there.")));
            }
            let engine = Engine::open_with(&data_dir, &personal_dir)?;
            let sites = if all { engine.sites() } else { pick_sites(&engine, &sites, &groups)? };
            let mut stderr = std::io::stderr();
            for site in &sites {
                let progress = |p: prospector_core::IndexProgress| {
                    if !quiet && (p.done == p.total || p.done.is_multiple_of(200)) {
                        let mut err = std::io::stderr();
                        let _ = write!(err, "\r{}: {:?} {}/{}   ", site.name, p.phase, p.done, p.total);
                        let _ = err.flush();
                    }
                };
                let updated = engine.index_site(&site.id, &site.excluded, &AtomicBool::new(false), &progress)?;
                let missing = if updated.missing_roots.is_empty() { String::new() } else { format!(", {} folder(s) missing", updated.missing_roots.len()) };
                let line = format!("{}: {} documents{missing}", updated.name, updated.doc_count);
                if quiet {
                    writeln!(stderr, "{line}")?;
                } else {
                    writeln!(stderr, "\r{line}{}", " ".repeat(20))?;
                }
            }
            lease.release();
            Ok(ExitCode::SUCCESS)
        }
        Command::UninstallCleanup => Ok(ExitCode::SUCCESS),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(failure) => {
            eprintln!("prospector-cli: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// clap checks its definitions only when a subcommand is used: all of
    /// them are checked here (two arguments named `folders` once, BUG-034).
    #[test]
    fn the_command_line_is_well_formed() {
        super::Cli::command().debug_assert();
    }
}
