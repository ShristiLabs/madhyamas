import SwiftUI

/// Manual proxy/API entry — mirrors the Android SettingsScreen.
struct SettingsScreen: View {
    @ObservedObject var viewModel: MainViewModel

    @State private var portText: String = ""

    var body: some View {
        Form {
            Section("Proxy") {
                TextField("Host", text: $viewModel.config.proxyHost)
                    .keyboardType(.URL)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                TextField("Port", text: $portText)
                    .keyboardType(.numberPad)
                Toggle("TLS to proxy (tls=1)", isOn: $viewModel.config.useTls)
            }
            Section("Server API") {
                TextField("API base URL (https://host/api)", text: Binding(
                    get: { viewModel.config.apiBaseUrl ?? "" },
                    set: { viewModel.config.apiBaseUrl = $0.isEmpty ? nil : $0 }
                ))
                .keyboardType(.URL)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            }
        }
        .navigationTitle("Server")
        .onAppear {
            portText = String(viewModel.config.proxyPort)
        }
        .onDisappear {
            if let port = Int(portText), (1...65535).contains(port) {
                viewModel.config.proxyPort = port
            }
            viewModel.saveConfig()
        }
    }
}
