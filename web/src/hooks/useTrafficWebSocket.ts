import { useCallback, useMemo, useState } from "react";
import { useWebSocket } from "./useWebSocket";
import type {
  TrafficEntrySnapshot,
  WsServerMessage,
  WsConnectionInfo,
} from "@/types/websocket";

interface UseTrafficWebSocketOptions {
  enabled?: boolean;
  onTrafficUpdate?: (entries: TrafficEntrySnapshot[]) => void;
}

interface UseTrafficWebSocketReturn {
  traffic: TrafficEntrySnapshot[];
  count: number;
  isLoading: boolean;
  connectionInfo: WsConnectionInfo;
  isConnected: boolean;
  clearLocalTraffic: () => void;
  connect: () => void;
  disconnect: () => void;
}

export function useTrafficWebSocket(
  options: UseTrafficWebSocketOptions = {}
): UseTrafficWebSocketReturn {
  const { enabled = true, onTrafficUpdate } = options;

  const [traffic, setTraffic] = useState<TrafficEntrySnapshot[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Build WebSocket URL (includes base path for context-path deployments)
  const wsUrl = useMemo(() => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    // Derive base path from the meta tag injected by the backend.
    let basePath = "/";
    const meta = document.querySelector('meta[name="madhyamas-base-path"]');
    const content = meta?.getAttribute("content");
    if (content && content.trim()) {
      let p = content.trim();
      if (!p.startsWith("/")) p = "/" + p;
      if (!p.endsWith("/")) p = p + "/";
      basePath = p;
    }
    return `${protocol}//${host}${basePath}api/ws`;
  }, []);

  const handleMessage = useCallback(
    (message: WsServerMessage) => {
      switch (message.type) {
        case "InitialTraffic":
          setTraffic(message.data);
          setIsLoading(false);
          onTrafficUpdate?.(message.data);
          break;

        case "Traffic": {
          const event = message.data;
          switch (event.type) {
            case "Added":
              setTraffic((prev) => {
                // Add to beginning (newest first)
                const updated = [event.data, ...prev];
                onTrafficUpdate?.(updated);
                return updated;
              });
              break;

            case "Updated":
              setTraffic((prev) => {
                const updated = prev.map((entry) =>
                  entry.id === event.data.id ? event.data : entry
                );
                onTrafficUpdate?.(updated);
                return updated;
              });
              break;

            case "Deleted":
              setTraffic((prev) => {
                const deletedIds = new Set(event.data);
                const updated = prev.filter(
                  (entry) => !deletedIds.has(entry.id)
                );
                onTrafficUpdate?.(updated);
                return updated;
              });
              break;

            case "Cleared":
              setTraffic([]);
              onTrafficUpdate?.([]);
              break;

            case "CountUpdate":
              // Count is derived from traffic array, but we could use this for validation
              break;
          }
          break;
        }

        case "Connected":
          // Connected with client ID
          break;

        case "Error":
          console.error("[TrafficWS] Server error:", message.data.message);
          break;

        case "Pong":
          // Heartbeat response
          break;
      }
    },
    [onTrafficUpdate]
  );

  const handleConnect = useCallback((_clientId: string) => {
    setIsLoading(true); // Will be set to false when InitialTraffic arrives
  }, []);

  const handleDisconnect = useCallback(() => {
    // Keep existing traffic data on disconnect
  }, []);

  const { connectionInfo, connect, disconnect, isConnected } = useWebSocket({
    url: wsUrl,
    onMessage: handleMessage,
    onConnect: handleConnect,
    onDisconnect: handleDisconnect,
    autoConnect: enabled,
    reconnect: true,
  });

  // Clear local traffic state
  const clearLocalTraffic = useCallback(() => {
    setTraffic([]);
  }, []);

  return {
    traffic,
    count: traffic.length,
    isLoading,
    connectionInfo,
    isConnected,
    clearLocalTraffic,
    connect,
    disconnect,
  };
}
