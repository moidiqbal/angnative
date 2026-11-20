//
// AngNativeTests.swift — iOS shell tests (run on a Mac with the Rust library linked).
//

import XCTest
@testable import AngNativeApp
@testable import AngNativeHost

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
}
