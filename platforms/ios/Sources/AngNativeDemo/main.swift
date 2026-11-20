//
// main.swift — AngNative iOS demo app entry point.
//
// Builds and runs the DemoViewController on the main thread.
//

import SwiftUI
import UIKit
import AngNativeApp

@main
struct AngNativeDemoApp: App {
    var body: some Scene {
        WindowGroup {
            DemoViewControllerWrapper()
        }
    }
}

/// SwiftUI wrapper around UIKit DemoViewController.
struct DemoViewControllerWrapper: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        do {
            return try DemoViewController()
        } catch {
            print("Failed to create DemoViewController: \(error)")
            return UIViewController()
        }
    }

    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}
