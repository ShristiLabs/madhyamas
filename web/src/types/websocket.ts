// WebSocket message types matching the backend Rust types

export interface TrafficEntrySnapshot {
  id: string;
  session_id: string;
  method: string;
  url: string;
  host: string;
  path: string;
  status_code: number | null;
  status_message: string | null;
  content_type: string | null;
  response_content_type: string | null;
  duration_ms: number | null;
  request_size: number;
  response_size: number | null;
  timestamp: string;
  modified: boolean;
  has_request_body: boolean;
  has_response_body: boolean;
  is_passthrough: boolean;
}

// Traffic events from server
export type TrafficEvent =
  | { type: "Added"; data: TrafficEntrySnapshot }
  | { type: "Updated"; data: TrafficEntrySnapshot }
  | { type: "Deleted"; data: string[] }
  | { type: "Cleared" }
  | { type: "CountUpdate"; data: number };

// Server messages
export type WsServerMessage =
  | { type: "Traffic"; data: TrafficEvent }
  | { type: "InitialTraffic"; data: TrafficEntrySnapshot[] }
  | { type: "Connected"; data: { client_id: string } }
  | { type: "Pong" }
  | { type: "Error"; data: { message: string } };

// Client messages
export type WsClientMessage =
  | { type: "Subscribe"; data?: { filter?: TrafficSubscriptionFilter } }
  | { type: "Unsubscribe" }
  | { type: "GetInitialTraffic"; data?: { limit?: number } }
  | { type: "Ping" };

export interface TrafficSubscriptionFilter {
  search?: string;
  method?: string;
  status_code?: string;
}

// WebSocket connection state
export type WsConnectionState =
  | "connecting"
  | "connected"
  | "disconnected"
  | "reconnecting";

export interface WsConnectionInfo {
  state: WsConnectionState;
  clientId: string | null;
  reconnectAttempts: number;
  lastConnectedAt: Date | null;
  lastError: string | null;
}
