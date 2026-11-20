//
// UIViewFactory.swift — UIKit implementation of the AngNative view factory.
//
// Rust-emitted PlatformOps are mapped to UIKit views. The owner (typically a
// DemoViewController) must keep this factory alive for as long as the host can
// emit callbacks; the C `user_data` is an unretained pointer back to self.
//

import UIKit
import AngNativeHost

public final class UIKitViewFactory {

    private var views: [UInt64: UIView] = [:]

    /// Register all 8 callbacks with the Rust host.
    public init(host: AngNativeHost) throws {
        let userData = Unmanaged<UIKitViewFactory>.passUnretained(self).toOpaque()

        let result = angnative_host_register_view_factory(
            host,
            { nodeId, viewTypePtr, ud in
                guard let factory = factory(from: ud), let viewTypePtr else { return }
                let viewType = String(cString: viewTypePtr)
                let view = factory.makeView(viewType: viewType)
                DispatchQueue.main.async { factory.views[nodeId] = view }
            },
            { nodeId, x, y, width, height, ud in
                guard let factory = factory(from: ud) else { return }
                DispatchQueue.main.async {
                    guard let view = factory.views[nodeId] else { return }
                    view.frame = CGRect(x: CGFloat(x), y: CGFloat(y),
                                        width: CGFloat(width), height: CGFloat(height))
                }
            },
            { nodeId, propsPtr, ud in
                guard let factory = factory(from: ud), let propsPtr else { return }
                let json = String(cString: propsPtr)
                guard let data = json.data(using: .utf8),
                      let props = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
                else { return }
                DispatchQueue.main.async { factory.applyProps(props, to: nodeId) }
            },
            { nodeId, textPtr, ud in
                guard let factory = factory(from: ud), let textPtr else { return }
                let text = String(cString: textPtr)
                DispatchQueue.main.async { factory.setText(text, on: nodeId) }
            },
            { parentId, childId, ud in
                guard let factory = factory(from: ud) else { return }
                DispatchQueue.main.async { factory.attach(childId, to: parentId, at: nil) }
            },
            { parentId, childId, index, ud in
                guard let factory = factory(from: ud) else { return }
                DispatchQueue.main.async { factory.attach(childId, to: parentId, at: Int(index)) }
            },
            { parentId, childId, ud in
                guard let factory = factory(from: ud) else { return }
                DispatchQueue.main.async {
                    guard let child = factory.views[childId] else { return }
                    child.removeFromSuperview()
                }
            },
            { nodeId, ud in
                guard let factory = factory(from: ud) else { return }
                DispatchQueue.main.async {
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
        DispatchQueue.main.async {
            for rect in rects {
                guard let id = (rect["node_id"] as? NSNumber)?.uint64Value,
                      let x = rect["x"] as? Double, let y = rect["y"] as? Double,
                      let w = rect["width"] as? Double, let h = rect["height"] as? Double,
                      let view = self.views[id]
                else { continue }
                view.frame = CGRect(x: CGFloat(x), y: CGFloat(y),
                                    width: CGFloat(w), height: CGFloat(h))
            }
        }
    }

    // MARK: - Private helpers

    private func factory(from ud: UnsafeMutableRawPointer?) -> UIKitViewFactory? {
        guard let ud else { return nil }
        return Unmanaged<UIKitViewFactory>.fromOpaque(ud).takeUnretainedValue()
    }

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
            if let ns = value as? NSNumber, let label = view as? UILabel {
                label.font = label.font.withSize(CGFloat(ns.floatValue))
            }
        case "fontWeight":
            if let str = value as? String, let label = view as? UILabel {
                label.font = UIFont.boldSystemFont(ofSize: label.font.pointSize)
                if str == "normal" { label.font = UIFont.systemFont(ofSize: label.font.pointSize) }
            }
        case "opacity":
            if let ns = value as? NSNumber { view.alpha = CGFloat(ns.floatValue) }
        case "visibility":
            if let str = value as? String { view.isHidden = str == "hidden" }
        case "cornerRadius":
            if let ns = value as? NSNumber { view.layer.cornerRadius = CGFloat(ns.floatValue) }
        default: break
        }
    }

    private func setText(_ text: String, on nodeId: UInt64) {
        guard let view = views[nodeId] else { return }
        if let label = view as? UILabel { label.text = text }
        else if let button = view as? UIButton { button.setTitle(text, for: .normal) }
        else if let field = view as? UITextField { field.text = text }
    }
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
