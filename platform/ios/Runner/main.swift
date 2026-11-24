// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
// A thin native shell. The Rust staticlib, built by the `day xcode-backend build` script
// phase, exports day_main, which boots UIApplicationMain.
import UIKit
import ObjectiveC.runtime

// Day's view controllers inherit UIKit's orientation policy. Install this app-local
// policy before UIApplicationMain, so UIKit sees it from its first controller query.
private var portraitBoard = false
private func installOrientationPolicy() {
    let selector = #selector(getter: UIViewController.supportedInterfaceOrientations)
    guard let method = class_getInstanceMethod(UIViewController.self, selector) else { return }
    // Store the immutable C entry point as an address when crossing the actor assertion.
    let original = unsafeBitCast(method_getImplementation(method), to: UInt.self)
    let replacement: @convention(block) (UIViewController) -> UInt = { controller in
        MainActor.assumeIsolated {
            if portraitBoard { return UIInterfaceOrientationMask.portrait.rawValue }
            typealias Original = @convention(c) (UIViewController, Selector) -> UInt
            return unsafeBitCast(original, to: Original.self)(controller, selector)
        }
    }
    method_setImplementation(method, imp_implementationWithBlock(replacement))
}

@_cdecl("games_fair_set_portrait")
func gamesFairSetPortrait(_ locked: Bool) {
    DispatchQueue.main.async {
        portraitBoard = locked
        for case let scene as UIWindowScene in UIApplication.shared.connectedScenes {
            if #available(iOS 16.0, *) {
                for window in scene.windows {
                    var controller = window.rootViewController
                    while let presented = controller?.presentedViewController { controller = presented }
                    controller?.setNeedsUpdateOfSupportedInterfaceOrientations()
                    window.rootViewController?.setNeedsUpdateOfSupportedInterfaceOrientations()
                }
                scene.requestGeometryUpdate(.iOS(interfaceOrientations: locked ? .portrait : .all))
            } else if locked {
                // iOS 15 has no scene geometry request API.
                UIDevice.current.setValue(UIInterfaceOrientation.portrait.rawValue, forKey: "orientation")
            }
        }
        UIViewController.attemptRotationToDeviceOrientation()
    }
}


@_silgen_name("day_main")
func day_main()

installOrientationPolicy()
day_main()
