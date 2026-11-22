#!/usr/bin/env python3
"""Find the tap coordinates of the LAST '+' text node in a uiautomator XML dump.

The ecosystem-core screen has an imperative '+' button and an Angular-driven
'+' text view. The Angular one is rendered last, so the final match is used.
Prints 'x y' or exits 1 when not found.
"""
import re
import sys


def main() -> int:
    if len(sys.argv) != 2:
        return 2
    with open(sys.argv[1], encoding="utf-8") as handle:
        xml = handle.read()
    matches = re.findall(
        r'<node[^>]*text="\+"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"',
        xml,
    )
    if not matches:
        return 1
    x1, y1, x2, y2 = map(int, matches[-1])
    print((x1 + x2) // 2, (y1 + y2) // 2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
