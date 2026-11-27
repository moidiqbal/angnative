//
// DemoViewController.swift — iOS demo rendered by the Rust host through UIKit.
//

import UIKit
import AngNativeHost

public final class DemoViewController: UIViewController {

    private let host: NativeHost
    private let factory: UIKitViewFactory
    private let containerView = UIView(frame: .zero)
    private var pressCount = 0

    /// Create the Rust host and register the UIKit view factory.
    /// (A throwing `init()` cannot override UIViewController's non-throwing
    /// `init()`, so construction goes through this factory method.)
    public static func make() throws -> DemoViewController {
        let host = try NativeHost()
        let factory = try UIKitViewFactory(host: host.handle)
        return DemoViewController(host: host, factory: factory)
    }

    private init(host: NativeHost, factory: UIKitViewFactory) {
        self.host = host
        self.factory = factory
        super.init(nibName: nil, bundle: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    public override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground
        containerView.frame = view.bounds
        containerView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(containerView)

        do {
            try loadDemoApp()
        } catch {
            print("[AngNative] demo load failed: \(error)")
        }
    }

    private func loadDemoApp() throws {
        let batch = DemoAppBuilder.buildBatch()
        _ = try host.processBatch(batch)

        let request = DemoAppBuilder.buildLayoutJSON(
            rootId: 1,
            width: Float(view.bounds.width),
            height: Float(view.bounds.height)
        )
        let layout = try host.runLayout(request)
        factory.applyLayout(layout)

        if let root = factory.view(for: 1) {
            root.frame = containerView.bounds
            root.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            containerView.addSubview(root)
        }

        for buttonId in [UInt64(5), 6, 7] {
            guard let button = factory.view(for: buttonId) as? UIButton else { continue }
            button.addTarget(self, action: #selector(onPress(_:)), for: .touchUpInside)
        }

        print("[AngNative] nodeCount=\(host.nodeCount)")
    }

    @objc private func onPress(_ sender: UIButton) {
        pressCount += 1
        let text = pressCount == 1 ? "Hello World!" : "Hello World! (x\(pressCount))"
        let update: [String: Any] = [
            "protocolVersion": 1,
            "frameId": UInt64(pressCount),
            "timestampMs": 0,
            "commands": [["kind": "SetTextContent", "id": 7, "text": text]]
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: update),
              let json = String(data: data, encoding: .utf8) else { return }
        do { _ = try host.processBatch(json) } catch { print("[AngNative] update failed: \(error)") }
    }
}

// MARK: - Demo command/layout builder

enum DemoAppBuilder {

    static func buildBatch() -> String {
        var commands: [[String: Any]] = []

        commands.append(["kind": "CreateNode", "id": 1, "nodeType": "View"])
        commands.append(["kind": "SetStyle", "id": 1, "key": "flexDirection", "value": "column"])
        commands.append(["kind": "SetStyle", "id": 1, "key": "padding", "value": "24px"])

        commands.append(["kind": "CreateNode", "id": 2, "nodeType": "Text"])
        commands.append(["kind": "SetTextContent", "id": 2, "text": "ANGNATIVE • IOS SHELL"])
        commands.append(["kind": "SetStyle", "id": 2, "key": "fontSize", "value": "12px"])
        commands.append(["kind": "SetStyle", "id": 2, "key": "color", "value": "#4338CA"])
        commands.append(["kind": "SetStyle", "id": 2, "key": "height", "value": "28px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 2, "index": 0])

        commands.append(["kind": "CreateNode", "id": 3, "nodeType": "Text"])
        commands.append(["kind": "SetTextContent", "id": 3, "text": "Hello World Demo"])
        commands.append(["kind": "SetStyle", "id": 3, "key": "fontSize", "value": "26px"])
        commands.append(["kind": "SetStyle", "id": 3, "key": "fontWeight", "value": "bold"])
        commands.append(["kind": "SetStyle", "id": 3, "key": "height", "value": "44px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 3, "index": 1])

        commands.append(["kind": "CreateNode", "id": 4, "nodeType": "Text"])
        commands.append(["kind": "SetTextContent", "id": 4, "text": "Angular running natively — no WebView."])
        commands.append(["kind": "SetStyle", "id": 4, "key": "fontSize", "value": "14px"])
        commands.append(["kind": "SetStyle", "id": 4, "key": "color", "value": "#64748B"])
        commands.append(["kind": "SetStyle", "id": 4, "key": "height", "value": "60px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 4, "index": 2])

        commands.append(["kind": "CreateNode", "id": 5, "nodeType": "Button"])
        commands.append(["kind": "SetTextContent", "id": 5, "text": "Click Me!"])
        commands.append(["kind": "SetStyle", "id": 5, "key": "height", "value": "50px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 5, "index": 3])

        commands.append(["kind": "CreateNode", "id": 6, "nodeType": "Button"])
        commands.append(["kind": "SetTextContent", "id": 6, "text": "Reset"])
        commands.append(["kind": "SetStyle", "id": 6, "key": "height", "value": "44px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 6, "index": 4])

        commands.append(["kind": "CreateNode", "id": 7, "nodeType": "Text"])
        commands.append(["kind": "SetTextContent", "id": 7, "text": ""])
        commands.append(["kind": "SetStyle", "id": 7, "key": "fontSize", "value": "18px"])
        commands.append(["kind": "SetStyle", "id": 7, "key": "color", "value": "#1D4ED8"])
        commands.append(["kind": "SetStyle", "id": 7, "key": "height", "value": "40px"])
        commands.append(["kind": "InsertChild", "parentId": 1, "childId": 7, "index": 5])

        let batch: [String: Any] = [
            "protocolVersion": 1,
            "frameId": 0,
            "timestampMs": 0,
            "commands": commands
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: batch),
              let json = String(data: data, encoding: .utf8) else { return "{}" }
        return json
    }

    static func buildLayoutJSON(rootId: UInt64, width: Float, height: Float) -> String {
        let nodes: [[String: Any]] = [
            ["node_id": 1,
             "styles": ["flexDirection": "column", "padding": "24px",
                        "width": "\(width)px", "height": "\(height)px"],
             "children": [2, 3, 4, 5, 6, 7]],
            ["node_id": 2, "styles": ["height": "28px"], "children": []],
            ["node_id": 3, "styles": ["height": "44px"], "children": []],
            ["node_id": 4, "styles": ["height": "60px"], "children": []],
            ["node_id": 5, "styles": ["height": "50px"], "children": []],
            ["node_id": 6, "styles": ["height": "44px"], "children": []],
            ["node_id": 7, "styles": ["height": "40px"], "children": []],
        ]
        let request: [String: Any] = [
            "root_id": rootId,
            "nodes": nodes,
            "available_width": width,
            "available_height": height
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: request),
              let json = String(data: data, encoding: .utf8) else { return "{}" }
        return json
    }
}
