//
// UIViewFactory.swift — UIKit implementation of the AngNative view factory.
//
// Rust-emitted PlatformOps are mapped to UIKit views. The owner (typically a
// DemoViewController) must keep this factory alive for as long as the host can
// emit callbacks; the C `user_data` is an unretained pointer back to self.
//

import UIKit
import AngNativeHost

/// The Rust host invokes these callbacks synchronously from inside
/// `angnative_host_process_batch` / `run_layout`, on the thread that called it.
/// The host and runtime bridge are thread-affine and must be driven from the
/// main thread, so each callback asserts main-actor isolation and mutates
/// UIKit synchronously. (Deferring with `DispatchQueue.main.async` meant the
/// views did not exist yet when the caller tried to mount the root after
/// `processBatch` returned.)
@MainActor
public final class UIKitViewFactory {

    // Internal (not private) getter so @testable tests can inspect the tree.
    private(set) var views: [UInt64: UIView] = [:]

    /// Register all 8 callbacks with the Rust host.
    public init(host: AngNativeHost) throws {
        let userData = Unmanaged<UIKitViewFactory>.passUnretained(self).toOpaque()

        let result = angnative_host_register_view_factory(
            host,
            { nodeId, viewTypePtr, ud in
                guard let viewTypePtr else { return }
                let viewType = String(cString: viewTypePtr)
                MainActor.assumeIsolated {
                    guard let factory = factoryFrom(ud) else { return }
                    factory.views[nodeId] = factory.makeView(viewType: viewType)
                }
            },
            { nodeId, x, y, width, height, ud in
                MainActor.assumeIsolated {
                    guard let view = factoryFrom(ud)?.views[nodeId] else { return }
                    view.frame = CGRect(x: CGFloat(x), y: CGFloat(y),
                                        width: CGFloat(width), height: CGFloat(height))
                }
            },
            { nodeId, propsPtr, ud in
                guard let propsPtr else { return }
                let json = String(cString: propsPtr)
                MainActor.assumeIsolated {
                    guard let factory = factoryFrom(ud),
                          let data = json.data(using: .utf8),
                          let props = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
                    else { return }
                    factory.applyProps(props, to: nodeId)
                }
            },
            { nodeId, textPtr, ud in
                guard let textPtr else { return }
                let text = String(cString: textPtr)
                MainActor.assumeIsolated {
                    factoryFrom(ud)?.setText(text, on: nodeId)
                }
            },
            { parentId, childId, ud in
                MainActor.assumeIsolated {
                    factoryFrom(ud)?.attach(childId, to: parentId, at: nil)
                }
            },
            { parentId, childId, index, ud in
                MainActor.assumeIsolated {
                    factoryFrom(ud)?.attach(childId, to: parentId, at: Int(index))
                }
            },
            { _, childId, ud in
                MainActor.assumeIsolated {
                    factoryFrom(ud)?.views[childId]?.removeFromSuperview()
                }
            },
            { nodeId, ud in
                MainActor.assumeIsolated {
                    guard let factory = factoryFrom(ud) else { return }
                    factory.views[nodeId]?.removeFromSuperview()
                    factory.views.removeValue(forKey: nodeId)
                }
            },
            userData
        )

        if result != 0 {
            throw NSError(domain: "AngNative", code: Int(result),
                          userInfo: [NSLocalizedDescriptionKey: "View factory registration failed"])
        }
    }

    /// Exposed so the demo can mount the root view and apply layout rects.
    public func view(for nodeId: UInt64) -> UIView? { views[nodeId] }

    /// Apply a LayoutResponseJson (`{"nodes":[{"node_id","x","y","width","height"}]}`).
    public func applyLayout(_ json: String) {
        guard let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let rects = obj["nodes"] as? [[String: Any]]
        else { return }
        for rect in rects {
            guard let id = (rect["node_id"] as? NSNumber)?.uint64Value,
                  let x = rect["x"] as? Double, let y = rect["y"] as? Double,
                  let w = rect["width"] as? Double, let h = rect["height"] as? Double,
                  let view = views[id]
            else { continue }
            view.frame = CGRect(x: CGFloat(x), y: CGFloat(y),
                                width: CGFloat(w), height: CGFloat(h))
        }
    }

    // MARK: - Private helpers

    private func makeView(viewType: String) -> UIView {
        switch viewType {
        case "Text":
            let label = UILabel(frame: .zero)
            label.numberOfLines = 0
            label.textColor = .label
            return label
        case "Button":
            let button = UIButton(type: .system)
            button.setTitleColor(.white, for: .normal)
            button.backgroundColor = UIColor(hex: "#2563EB")
            button.layer.cornerRadius = 10
            return button
        case "TextInput": return UITextField(frame: .zero)
        case "Image": return UIImageView(frame: .zero)
        case "ScrollView": return UIScrollView(frame: .zero)
        case "Switch": return UISwitch(frame: .zero)
        case "ActivityIndicator": return UIActivityIndicatorView(frame: .zero)
        default: return UIView(frame: .zero)
        }
    }

    private func attach(_ childId: UInt64, to parentId: UInt64, at index: Int?) {
        guard let parent = views[parentId], let child = views[childId] else { return }
        if let index, index >= 0, index <= parent.subviews.count {
            parent.insertSubview(child, at: index)
        } else {
            parent.addSubview(child)
        }
    }

    private func applyProps(_ props: [String: Any], to nodeId: UInt64) {
        guard let view = views[nodeId] else { return }
        if let styles = props["styles"] as? [String: Any] {
            for (key, value) in styles { apply(style: key, value: value, to: view) }
        }
    }

    private func apply(style: String, value: Any, to view: UIView) {
        switch style {
        case "backgroundColor":
            if let str = value as? String { view.backgroundColor = UIColor(hex: str) }
        case "color":
            if let str = value as? String, let label = view as? UILabel {
                label.textColor = UIColor(hex: str)
            }
        case "fontSize":
            if let size = cgFloat(value), let label = view as? UILabel {
                label.font = label.font.withSize(size)
            }
        case "fontWeight":
            if let str = value as? String, let label = view as? UILabel {
                label.font = UIFont.boldSystemFont(ofSize: label.font.pointSize)
                if str == "normal" { label.font = UIFont.systemFont(ofSize: label.font.pointSize) }
            }
        case "opacity":
            if let alpha = cgFloat(value) { view.alpha = alpha }
        case "visibility":
            if let str = value as? String { view.isHidden = str == "hidden" }
        case "cornerRadius":
            if let radius = cgFloat(value) { view.layer.cornerRadius = radius }
        default: break
        }
    }

    /// Rust serialises styles as strings (`"12px"`, `"0.5"`); accept numbers too.
    private func cgFloat(_ value: Any) -> CGFloat? {
        if let ns = value as? NSNumber { return CGFloat(ns.doubleValue) }
        guard let str = value as? String else { return nil }
        let trimmed = str.trimmingCharacters(in: .whitespaces)
        let numeric = trimmed.hasSuffix("px") ? String(trimmed.dropLast(2)) : trimmed
        return Double(numeric).map { CGFloat($0) }
    }

    private func setText(_ text: String, on nodeId: UInt64) {
        guard let view = views[nodeId] else { return }
        if let label = view as? UILabel { label.text = text }
        else if let button = view as? UIButton { button.setTitle(text, for: .normal) }
        else if let field = view as? UITextField { field.text = text }
    }
}

/// Recover the factory from the C `user_data` pointer. A free function, since
/// `@convention(c)` closures cannot capture context.
private func factoryFrom(_ ud: UnsafeMutableRawPointer?) -> UIKitViewFactory? {
    guard let ud else { return nil }
    return Unmanaged<UIKitViewFactory>.fromOpaque(ud).takeUnretainedValue()
}

// MARK: - UIColor hex helper

extension UIColor {
    convenience init(hex: String) {
        var sanitized = hex.trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: "#", with: "")
        if sanitized.count == 3 {
            sanitized = sanitized.map { "\($0)\($0)" }.joined()
        }
        var rgb: UInt64 = 0
        Scanner(string: sanitized).scanHexInt64(&rgb)
        self.init(
            red: CGFloat((rgb & 0xFF0000) >> 16) / 255,
            green: CGFloat((rgb & 0x00FF00) >> 8) / 255,
            blue: CGFloat(rgb & 0x0000FF) / 255,
            alpha: 1
        )
    }
}
