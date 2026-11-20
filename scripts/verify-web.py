"""Playwright verification for the AngNative web adapter demo."""
from playwright.sync_api import sync_playwright

BASE = r"C:\Users\Moid Iqbal\Projects\angnative\docs\assets"

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1280, "height": 900})
    logs = []
    page.on("console", lambda m: logs.append(f"{m.type}: {m.text}"))
    page.on("pageerror", lambda e: logs.append(f"pageerror: {e}"))

    page.goto("http://localhost:4173")
    page.wait_for_load_state("networkidle")
    page.wait_for_timeout(2000)

    print("TITLE:", page.title())
    print("BODY:", page.locator("body").inner_text()[:800].replace("\n", " | "))
    print("BUTTONS:", [b.inner_text() for b in page.locator("button").all()])
    print("NA-TAGS:", page.locator("na-view, na-text, na-button").count())
    page.screenshot(path=BASE + r"\web-demo.png", full_page=True)

    buttons = page.locator("na-button").all()
    print("NA-BUTTONS:", len(buttons))
    if buttons:
        buttons[-1].click()  # the '+' button
        page.wait_for_timeout(1000)
        print("BODY AFTER CLICK:", page.locator("body").inner_text()[:800].replace("\n", " | "))
        page.screenshot(path=BASE + r"\web-demo-clicked.png", full_page=True)

    print("LOGS:", [line.encode("unicode_escape").decode("ascii") for line in logs[:25]])
    browser.close()
