import SwiftUI
import MadhyamasCore

@main
struct MadhyamasApp: App {
    @StateObject private var viewModel = MainViewModel()

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                MainScreen(viewModel: viewModel)
            }
            .onOpenURL { url in
                viewModel.handleDeepLink(url)
            }
        }
    }
}
