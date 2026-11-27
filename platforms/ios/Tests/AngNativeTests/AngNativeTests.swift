//
// AngNativeTests.swift — iOS shell tests (run on a Mac with the Rust library linked).
//

import UIKit
import XCTest
@testable import AngNativeApp
@testable import AngNativeHost

// The host and UIKit view factory are main-thread only (see UIKitViewFactory).
@MainActor
final class AngNativeTests: XCTestCase {

    func testNativeHostCreation() throws {
        let host = try NativeHost()
        XCTAssertNotNil(host.handle)
        XCTAssertGreaterThanOrEqual(host.nodeCount, 0)
        host.destroy()
    }

    func testBatchProcessing() throws {
        let host = try NativeHost()
        defer { host.destroy() }

        let ops = try host.processBatch(DemoAppBuilder.buildBatch())
        let opsData = ops.data(using: .utf8)!
        let array = try JSONSerialization.jsonObject(with: opsData) as? [Any]
        XCTAssertNotNil(array)
        XCTAssertGreaterThan(array?.count ?? 0, 0)
    }

    func testNodeCountAndSnapshot() throws {
        let host = try NativeHost()
        defer { host.destroy() }

        XCTAssertEqual(host.nodeCount, 0)
        _ = try host.processBatch(DemoAppBuilder.buildBatch())
        XCTAssertGreaterThan(host.nodeCount, 0)

        let snapshot = try host.snapshot()
        let json = try JSONSerialization.jsonObject(with: snapshot.data(using: .utf8)!) as? [String: Any]
        let nodes = json?["nodes"] as? [[String: Any]]
        XCTAssertGreaterThan(nodes?.count ?? 0, 0)
    }

    func testLayoutComputation() throws {
        let host = try NativeHost()
        defer { host.destroy() }

        _ = try host.processBatch(DemoAppBuilder.buildBatch())
        let result = try host.runLayout(
            DemoAppBuilder.buildLayoutJSON(rootId: 1, width: 390, height: 844)
        )
        let json = try JSONSerialization.jsonObject(with: result.data(using: .utf8)!) as? [String: Any]
        let rects = json?["nodes"] as? [[String: Any]]
        XCTAssertGreaterThan(rects?.count ?? 0, 0)
    }

    func testViewFactoryRegistration() throws {
        let host = try NativeHost()
        defer { host.destroy() }

        let factory = try UIKitViewFactory(host: host.handle)
        XCTAssertNotNil(factory)
        _ = try host.processBatch(DemoAppBuilder.buildBatch())
        XCTAssertGreaterThan(host.nodeCount, 0)
    }

    /// The Rust host calls the view factory synchronously inside
    /// processBatch, so the UIKit tree must exist as soon as it returns.
    /// (Regression: callbacks used to defer via DispatchQueue.main.async,
    /// leaving the demo screen blank and its buttons unwired.)
    func testFactoryBuildsUIKitTreeSynchronously() throws {
        let host = try NativeHost()
        defer { host.destroy() }
        let factory = try UIKitViewFactory(host: host.handle)

        _ = try host.processBatch(DemoAppBuilder.buildBatch())

        let root = try XCTUnwrap(factory.view(for: 1), "root view must exist after processBatch")
        XCTAssertEqual(root.subviews.count, 6)

        let badge = try XCTUnwrap(factory.view(for: 2) as? UILabel)
        XCTAssertEqual(badge.text, "ANGNATIVE • IOS SHELL")
        XCTAssertTrue(badge.superview === root)
        // Rust serialises styles as strings ("12px").
        XCTAssertEqual(badge.font.pointSize, 12, accuracy: 0.01)

        let button = try XCTUnwrap(factory.view(for: 5) as? UIButton)
        XCTAssertEqual(button.title(for: .normal), "Click Me!")

        // A follow-up SetTextContent batch reaches the UILabel.
        let update = #"{"protocolVersion":1,"frameId":1,"timestampMs":0,"#
            + #""commands":[{"kind":"SetTextContent","id":7,"text":"Hello World!"}]}"#
        _ = try host.processBatch(update)
        XCTAssertEqual((factory.view(for: 7) as? UILabel)?.text, "Hello World!")
    }

    /// The shipped demo: loading the view controller mounts the Rust-built
    /// UIKit tree, and a button's touchUpInside sends a SetTextContent batch
    /// through Rust that updates the output UILabel.
    func testDemoViewControllerButtonUpdatesOutputLabel() throws {
        let controller = try DemoViewController.make()
        controller.view.frame = CGRect(x: 0, y: 0, width: 390, height: 844)
        controller.loadViewIfNeeded()

        let tree = controller.view.allSubviews
        let button = try XCTUnwrap(
            tree.compactMap { $0 as? UIButton }.first { $0.title(for: .normal) == "Click Me!" },
            "demo button must be mounted in the view hierarchy")
        let labels = tree.compactMap { $0 as? UILabel }
        XCTAssertTrue(labels.contains { $0.text == "Hello World Demo" })
        XCTAssertFalse(labels.contains { $0.text == "Hello World!" })

        // Hostless SwiftPM tests have no UIApplication, so UIKit cannot deliver
        // sendActions(for:). Verify the registered touchUpInside action and
        // invoke exactly that selector on its target instead.
        let actions = try XCTUnwrap(
            button.actions(forTarget: controller, forControlEvent: .touchUpInside),
            "button must have the controller's touchUpInside action registered")
        XCTAssertEqual(actions.count, 1)
        _ = controller.perform(NSSelectorFromString(actions[0]), with: button)

        XCTAssertTrue(labels.contains { $0.text == "Hello World!" },
                      "tap must update the output label via Rust SetTextContent")
    }

    /// End-to-end on iOS: a real Angular AOT bundle boots in QuickJS inside
    /// the unified runtime bridge, its renderer batches become UIKit views,
    /// and a native click reaches the Angular listener and updates a UILabel.
    ///
    /// Uses dist/aot/quickjs-bundle.js when generated locally
    /// (`pnpm compile:aot && pnpm bundle:quickjs`), otherwise the committed
    /// copy CI ships to the Android ecosystem-core showcase.
    func testRuntimeBridgeBootsAngularBundleIntoUIKit() throws {
        let bundleURL = try XCTUnwrap(Self.angularBundleURL(),
            "no Angular bundle found; run pnpm compile:aot && pnpm bundle:quickjs")
        let source = try String(contentsOf: bundleURL, encoding: .utf8)

        // Only one bridge per process: create and destroy it in this test.
        let bridge = try NativeRuntimeBridge()
        defer { bridge.destroy() }
        let host = try bridge.hostHandle()  // borrowed; the bridge owns it
        let factory = try UIKitViewFactory(host: host)

        XCTAssertEqual(try bridge.loadScript(source), 0, bridge.lastErrorJson() ?? "")
        XCTAssertGreaterThan(angnative_host_node_count(host), 0)
        XCTAssertFalse(factory.views.isEmpty, "Angular renderer must create UIKit views")

        let labelsBefore = Self.labelTexts(factory)
        XCTAssertFalse(labelsBefore.isEmpty, "Angular text must reach UILabels")

        // Find the Angular (click) target among the native buttons.
        let buttonIds = factory.views
            .filter { $0.value is UIButton }
            .map(\.key)
            .sorted()
        XCTAssertFalse(buttonIds.isEmpty, "Angular <na-button> must map to UIButton")

        var fired: UInt64?
        for id in buttonIds {
            let result = bridge.dispatchEvent(
                nodeId: id, eventName: "click", payloadJson: #"{"source":"native"}"#)
            XCTAssertFalse(result.rejectedForWrongThread)
            if result.listenersInvoked == 1 { fired = id; break }
        }
        XCTAssertNotNil(fired, "a native click must reach exactly one Angular listener")
        let labelsAfter = Self.labelTexts(factory)
        print("[AngNative] bundle=\(bundleURL.lastPathComponent) nodes=\(angnative_host_node_count(host))"
              + " views=\(factory.views.count) buttons=\(buttonIds) clicked=\(fired.map(String.init) ?? "none")")
        for id in labelsBefore.keys.sorted() where labelsBefore[id] != labelsAfter[id] {
            print("[AngNative]   label \(id): \(labelsBefore[id] ?? "") -> \(labelsAfter[id] ?? "")")
        }
        XCTAssertNotEqual(labelsAfter, labelsBefore,
                          "the Angular handler must update native UILabel text")
    }

    private static func labelTexts(_ factory: UIKitViewFactory) -> [UInt64: String] {
        factory.views.compactMapValues { ($0 as? UILabel)?.text }
    }

    /// The Simulator can read host paths, so locate the bundle from this file.
    private static func angularBundleURL() -> URL? {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { root.deleteLastPathComponent() }  // -> repo root
        return [
            "dist/aot/quickjs-bundle.js",
            "showcases/ecosystem-core/app/src/main/assets/angnative-bundle.js",
        ]
        .map { root.appendingPathComponent($0) }
        .first { FileManager.default.fileExists(atPath: $0.path) }
    }
}

private extension UIView {
    var allSubviews: [UIView] { subviews + subviews.flatMap(\.allSubviews) }
}
