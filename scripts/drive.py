"""Drive the running development app (`pnpm tauri dev`) for the manual tests.

The debug build opens the Chrome DevTools Protocol on 127.0.0.1:9222
(src-tauri/src/lib.rs). This connects to that window with Playwright: the real
engine, the real indexes. Nothing is launched here: the app must be open.

    python scripts/drive.py shot NAME            screenshot → target/tmp/drive/NAME.png
    python scripts/drive.py eval "JS"            evaluate JavaScript, print the result
    python scripts/drive.py search "QUERY"       type a query, press Enter, print the summary
    python scripts/drive.py click "SELECTOR"     click an element (Playwright selector)

Or import it: `with app() as page: ...`.
"""
import contextlib
import json
import pathlib
import sys

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parent.parent
SHOTS = ROOT / "target" / "tmp" / "drive"
CDP = "http://127.0.0.1:9222"


@contextlib.contextmanager
def app():
    """The page of the Prospector window."""
    with sync_playwright() as p:
        browser = p.chromium.connect_over_cdp(CDP)
        pages = [pg for ctx in browser.contexts for pg in ctx.pages]
        page = next((pg for pg in pages if "localhost" in pg.url or "tauri" in pg.url), pages[0] if pages else None)
        if page is None:
            raise SystemExit("no window found: is `pnpm tauri dev` running?")
        yield page
        # Leave the app running: only the connection is closed.
        browser.close()


def shot(page, name):
    SHOTS.mkdir(parents=True, exist_ok=True)
    path = SHOTS / f"{name}.png"
    page.screenshot(path=str(path))
    return path


def search(page, query):
    box = page.locator("input[type=search], input.query, form input[type=text]").first
    box.fill(query)
    box.press("Enter")
    page.wait_for_timeout(300)
    page.wait_for_function("() => !document.querySelector('[aria-busy=true]')", timeout=30_000)
    page.wait_for_timeout(200)
    return page.evaluate(
        "() => [...document.querySelectorAll('h2, .summary, .count')].map(e => e.textContent.trim()).filter(Boolean).slice(0, 4)"
    )


def main(argv):
    sys.stdout.reconfigure(encoding="utf-8")
    if len(argv) < 2:
        raise SystemExit(__doc__)
    command, rest = argv[1], argv[2:]
    with app() as page:
        if command == "shot":
            print(shot(page, rest[0] if rest else "app"))
        elif command == "eval":
            print(json.dumps(page.evaluate(rest[0]), ensure_ascii=False, indent=2))
        elif command == "search":
            print(json.dumps(search(page, rest[0]), ensure_ascii=False, indent=2))
        elif command == "click":
            page.locator(rest[0]).first.click()
            page.wait_for_timeout(300)
            print("clicked")
        else:
            raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
