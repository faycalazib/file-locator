# Changelog

## Unreleased

- **File names and folders**: a file-name criterion (`*.pdf; facture-*`, `!exclude`, `/regex/`), alone or with words, and a search for folders.
- **Text in images and scanned PDFs** (Windows OCR, French, English, Spanish, Arabic).
- **More formats**: Word 97-2003 (.doc), PowerPoint 97-2003 (.ppt), RTF, OpenDocument (.odt, .odp), EPUB; TAR, GZ, BZ2 and JAR archives; e-mail attachments (.msg, .eml, .pst, .ost) and Thunderbird mailboxes (mbox).
- **Query language**: `a NEAR b` / `NEAR:20`, `LIKE word`, `LINES:3-5`, multi-line regular expressions.
- **Working with results**
  - Tabs (Ctrl+T, Ctrl+W, Ctrl+Tab), and a live scan that keeps running in a hidden tab.
  - History of each tab: Alt+← / Alt+→.
  - Instant filter of the list, and "Search within these results".
  - Save results to a `.prospector` file and open them again (Ctrl+S / Ctrl+O).
- **Images**: thumbnails in the results; in the preview, the picture with the words found boxed right on it, and the text read below.
- **Meaning search module** (first part): install the optional local AI module (77 MB, runs on your processor, nothing leaves the PC) from Settings; meaning search itself comes next.
- **Term lists**: search for a list of terms read from a text or CSV file (at least one, or all of them), with a count per term in the export and the PDF report; `--terms-file` on the command line.
- **Shared index**: put the index folder on a network share; one PC keeps it up to date, the others search it and see its updates at once, and another PC takes over if it is switched off. Saved searches, alerts and groups stay on each PC.
- **Site groups**: name a set of sites (“Clients”, “Code”) and switch to it in one click or with Ctrl+1…9 (Ctrl+0: every site); the command line takes `--group` too.
- **Command line**: `prospector-cli` searches the same sites from a script or a terminal (`search`, `scan`, `index`, `sites`), with every filter of the app, and prints text, JSON, JSON Lines or CSV; a click in Settings puts it in your PATH.
- **Portable version**: a ZIP to unpack on a USB drive; settings and indexes stay on the drive, nothing is written on the PC, and when the drive gets another letter on another PC, everything follows at once, without indexing again.
- **Extract from archives and e-mails**: "Extract to…" a file found inside a ZIP, 7z, RAR or TAR archive, or attached to an e-mail (.msg, .eml, Outlook .pst, Thunderbird mbox); "Open" opens that file itself; the bulk copy extracts them too; images inside archives and e-mails get their picture.
- **Duplicates**: identical files (even under another name) and near-identical documents (even across formats), with the space you would reclaim; send the extra copies to the Recycle Bin, safely.
- **Exports and bulk copy**: export to Excel (opens directly, right separator for your language), an HTML page or JSON, with the columns you choose and a keyword report (occurrences of each term per file); copy all the files found to a folder or a ZIP, keeping their folders if you want, without overwriting anything.
- **Counts per criterion**: how many files per kind, language, year and site, shown on the filters themselves (each counted without its own filter); a click filters.
- **Detectors and personal-data audit**: 13 kinds of data (IBAN, BIC, French RIB, bank card, e-mail, phone, IP address, amount, VAT number, SIREN/SIRET, French social security number, Spanish DNI/NIE, passport / ID card), each checked by its key, so random numbers are not reported. One click audits everything, with totals per kind and a PDF report where account and identity numbers are masked.
- **Alerts**: a bell on a saved search; Windows notifies you as soon as a new file matches, even for changes made while Prospector was closed. Prospector can keep running near the clock and start with Windows.
- **Dates, attributes, digests**
  - Date filter on the modification date, the creation date or the last access, with presets or a custom period.
  - Attributes: read-only, hidden (live scan also walks hidden files), system.
  - MD5 / SHA-256 digest criterion, and "Find copies" of a result.
  - Existing indexes are updated from their stored text: nothing is read again.
- **Windows integration**
  - One instance only: launching Prospector again brings its window back.
  - "Search with Prospector" in the Explorer's right-click menu (folder, folder background, drive): instant when the folder is in an indexed site, otherwise a live scan of that folder. Can be turned off in Settings.
  - "Open at line N" in your code editor (VS Code, VSCodium, Cursor, Notepad++, Sublime Text, or a command of your own).
  - `.prospector` files open in Prospector with a double-click.

## 0.1.0 — first public version (Windows)

- **Search**
  - Full-text search in English, French, Spanish and Arabic. Accents and Arabic vowels are ignored, plurals are grouped, and typos are tolerated (approximate matches are shown apart).
  - Query language: words, "exact phrase", OR, NOT / -word, /regular expression/. Options Aa (match case), ab (whole word), .* (whole input as a regular expression).
- **Formats**: text and code, PDF, Word, Excel, PowerPoint, e-mails (.eml, .msg, .pst), ZIP, RAR and 7z archives (nested ones included).
- **Indexing and scanning**
  - Indexed search in milliseconds.
  - Live scan without an index.
  - Watched folders updated automatically, including changes made while the app was closed.
- **Preview**: highlighted matches with ↑ ↓ / F3 navigation. Excel sheets as tables, PowerPoint slides as cards, code in color.
- **Your searches**
  - Filters: type, size, date, document language, excluded folders.
  - Saved searches (★) and recent searches.
  - Export as CSV or JSON, or as a PDF report.
- **Interface**
  - Global shortcut Ctrl+Shift+Space.
  - Themes "Colored pencils" and "Sonar", three layouts.
  - English, French, Spanish and Arabic (right to left).
- **Installers**: Windows installers in 4 languages, and automatic updates.
