import SwiftUI
import MadhyamasCore

/// Status + pairing card + VPN toggle — mirrors the Android MainScreen.
struct MainScreen: View {
    @ObservedObject var viewModel: MainViewModel
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        List {
            statusSection
            pairingSection
            caSection
            forgetSection
        }
        .navigationTitle("Madhyamas")
        .onChange(of: scenePhase) { phase in
            if phase == .active {
                viewModel.refreshStatus()
            }
        }
    }

    private var statusSection: some View {
        Section {
            Toggle("Capture traffic", isOn: Binding(
                get: { viewModel.vpnActive },
                set: { on in
                    if on {
                        viewModel.startVPN()
                    } else {
                        viewModel.stopVPN()
                    }
                }
            ))
            .disabled(viewModel.paired == nil)

            if viewModel.breakerTripped {
                Label("Proxy rejected the device key 3× — rotate or re-pair the device",
                      systemImage: "exclamationmark.triangle.fill")
                    .font(.footnote)
                    .foregroundStyle(.orange)
            }

            LabeledContent("Proxy", value: "\(viewModel.config.proxyHost):\(viewModel.config.proxyPort)")
            LabeledContent("TLS", value: viewModel.config.useTls ? "on" : "off")
            if let name = viewModel.paired?.deviceName {
                LabeledContent("Device", value: name)
            }
        } header: {
            Text("Status")
        }
    }

    private var pairingSection: some View {
        Section {
            if viewModel.enrollState == .enrolling {
                ProgressView("Redeeming enrollment token…")
            }
            if let message = viewModel.pairingMessage {
                Text(message).font(.footnote)
            }
            NavigationLink("Manual server settings") {
                SettingsScreen(viewModel: viewModel)
            }
        } header: {
            Text("Pairing")
        } footer: {
            Text("Scan the Devices-panel QR with the system Camera — the madhyamas://connect link opens this app and pairs it. While capture is on, all TCP traffic is routed through the proxy (per-app selection requires MDM on iOS).")
        }
    }

    private var caSection: some View {
        Section {
            Button("Download interception CA profile") {
                if let api = viewModel.config.apiBaseUrl,
                   let url = URL(string: api + "/cert/ca") {
                    UIApplication.shared.open(url)
                }
            }
        } header: {
            Text("HTTPS interception")
        } footer: {
            Text("Opens the CA in Safari. Install the profile (Settings → Profile Downloaded), then enable full trust in Settings → General → About → Certificate Trust Settings.")
        }
    }

    private var forgetSection: some View {
        Section {
            Button("Forget this device", role: .destructive) {
                viewModel.forget()
            }
            .disabled(viewModel.paired == nil)
        }
    }

}
