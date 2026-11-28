#!/usr/bin/env python3
"""Find the tap coordinates of the Angular '+' in a uiautomator XML dump.

The ecosystem-core screen has an imperative '+' button and an Angular-driven
'+' text view. The Angular view is labelled `angular-plus` by NativeViewFactory
so this proof cannot accidentally tap the imperative button. The legacy
fallback keeps older evidence dumps usable: it takes the last TextView '+'
(not a Button), which was the Angular view before the label was added.
Prints 'x y' or exits 1 when no Angular plus is present.
"""
import re
import sys

NODE_RE = re.compile(r"<node\b[^>]*>")
ATTR_RE = re.compile(r'([A-Za-z0-9_-]+)="([^"]*)"')
BOUNDS_RE = re.compile(r"\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]")


def node_attrs(tag: str) -> dict[str, str]:
    return dict(ATTR_RE.findall(tag))


def center(node: dict[str, str]) -> tuple[int, int]:
    match = BOUNDS_RE.fullmatch(node.get("bounds", ""))
    if not match:
        raise ValueError(f"node has no usable bounds: {node}")
    x1, y1, x2, y2 = (int(value) for value in match.groups())
    return (x1 + x2) // 2, (y1 + y2) // 2


def main() -> int:
    if len(sys.argv) != 2:
        return 2
    with open(sys.argv[1], encoding="utf-8") as handle:
        nodes = [node_attrs(tag) for tag in NODE_RE.findall(handle.read())]

    labelled = [node for node in nodes if node.get("content-desc") == "angular-plus"]
    if not labelled:
        labelled = [
            node
            for node in nodes
            if node.get("text") == "+" and node.get("class") != "android.widget.Button"
        ]
    if not labelled:
        print("no Angular '+' node found in the UI dump", file=sys.stderr)
        return 1

    x, y = center(labelled[-1])
    print(x, y)
    return 0


if __name__ == "__main__":
    sys.exit(main())
