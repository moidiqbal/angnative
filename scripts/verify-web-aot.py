"""Playwright smoke check for the AOT web demo.

Run via scripts/with_server.py so the local demo server lifecycle is managed:
  python scripts/with_server.py --server "node scripts/serve-web.mjs" --port 4173 -- python scripts/verify-web-aot.py
"""

import json
from playwright.sync_api import sync_playwright

URL = "http://localhost:4173/"
SCREENSHOT = "docs/assets/web-aot-demo.png"


def main() -> None:
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        console = []
        page_errors = []
        requests = []

        page.on("console", lambda msg: console.append(f"{msg.type}: {msg.text}"))
        page.on("pageerror", lambda err: page_errors.append(str(err)))
        page.on("request", lambda req: requests.append(req.url))

        page.goto(URL, wait_until="networkidle")
        page.wait_for_selector("demo-counter", timeout=20000)
        page.screenshot(path=SCREENSHOT, full_page=True)

        before = page.locator("body").inner_text()
        plus = page.locator("na-button", has_text="+")
        plus_count = plus.count()
        if plus_count:
            plus.first.click()
            page.wait_for_timeout(500)
        after = page.locator("body").inner_text()

        result = {
            "title": page.title(),
            "html_lang": page.locator("html").get_attribute("lang"),
            "script_fetchpriority": page.locator("script[src='/app.js']").get_attribute(
                "fetchpriority"
            ),
            "script_type": page.locator("script[src='/app.js']").get_attribute("type"),
            "counter_nodes": page.locator("demo-counter").count(),
            "na_view_nodes": page.locator("na-view").count(),
            "na_text_nodes": page.locator("na-text").count(),
            "na_button_nodes": page.locator("na-button").count(),
            "plus_button_count": plus_count,
            "text_before": before,
            "text_after": after,
            "requests": requests,
            "console": console,
            "page_errors": page_errors,
            "screenshot": SCREENSHOT,
        }
        print(json.dumps(result, indent=2, ensure_ascii=True))
        browser.close()

        if page_errors:
            raise SystemExit(1)
        if plus_count and before == after:
            raise SystemExit(2)


if __name__ == "__main__":
    main()
