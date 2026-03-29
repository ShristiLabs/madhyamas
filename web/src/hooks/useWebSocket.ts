import { useCallback, useEffect, useRef, useState } from "react";
import type {
  WsConnectionInfo,
  WsServerMessage,
  WsClientMessage,
} from "@/types/websocket";

const DEFAULT_RECONNECT_INTERVAL = 1000;
const MAX_RECONNECT_INTERVAL = 30000;
const MAX_RECONNECT_ATTEMPTS = 10;

interface UseWebSocketOptions {
  url: string;
  onMessage?: (message: WsServerMessage) => void;
  onConnect?: (clientId: string) => void;
  onDisconnect?: () => void;
  onError?: (error: Event) => void;
  autoConnect?: boolean;
  reconnect?: boolean;
  maxReconnectAttempts?: number;
}

interface UseWebSocketReturn {
  connectionInfo: WsConnectionInfo;
  connect: () => void;
  disconnect: () => void;
  send: (message: WsClientMessage) => void;
  isConnected: boolean;
}

export function useWebSocket(options: UseWebSocketOptions): UseWebSocketReturn {
  const {
    url,
    onMessage,
    onConnect,
    onDisconnect,
    onError,
    autoConnect = true,
    reconnect = true,
    maxReconnectAttempts = MAX_RECONNECT_ATTEMPTS,
  } = options;

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const isManualDisconnectRef = useRef(false);

  const [connectionInfo, setConnectionInfo] = useState<WsConnectionInfo>({
    state: "disconnected",
    clientId: null,
    reconnectAttempts: 0,
    lastConnectedAt: null,
    lastError: null,
  });

  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
  }, []);

  const connectRef = useRef<() => void>(() => {});

  const scheduleReconnect = useCallback(() => {
    if (!reconnect || isManualDisconnectRef.current) {
      return;
    }

    if (reconnectAttemptsRef.current >= maxReconnectAttempts) {
      setConnectionInfo((prev) => ({
        ...prev,
        state: "disconnected",
        lastError: "Max reconnect attempts reached",
      }));
      return;
    }

    // Exponential backoff with jitter
    const baseDelay =
      DEFAULT_RECONNECT_INTERVAL *
      Math.pow(2, reconnectAttemptsRef.current);
    const jitter = Math.random() * 1000;
    const delay = Math.min(baseDelay + jitter, MAX_RECONNECT_INTERVAL);

    setConnectionInfo((prev) => ({
      ...prev,
      state: "reconnecting",
      reconnectAttempts: reconnectAttemptsRef.current + 1,
    }));

    reconnectTimeoutRef.current = setTimeout(() => {
      reconnectAttemptsRef.current++;
      connectRef.current();
    }, delay);
  }, [reconnect, maxReconnectAttempts]);

  const connect = useCallback(() => {
    // Clean up existing connection
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    clearReconnectTimeout();
    isManualDisconnectRef.current = false;

    setConnectionInfo((prev) => ({
      ...prev,
      state: "connecting",
      lastError: null,
    }));

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        reconnectAttemptsRef.current = 0;
        setConnectionInfo((prev) => ({
          ...prev,
          state: "connected",
          reconnectAttempts: 0,
          lastConnectedAt: new Date(),
          lastError: null,
        }));
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WsServerMessage;

          // Handle connection acknowledgment
          if (message.type === "Connected") {
            setConnectionInfo((prev) => ({
              ...prev,
              clientId: message.data.client_id,
            }));
            onConnect?.(message.data.client_id);
          }

          onMessage?.(message);
        } catch (e) {
          console.error("[WebSocket] Failed to parse message:", e);
        }
      };

      ws.onerror = (event) => {
        console.error("[WebSocket] Error:", event);
        setConnectionInfo((prev) => ({
          ...prev,
          lastError: "WebSocket error",
        }));
        onError?.(event);
      };

      ws.onclose = (event) => {
        wsRef.current = null;

        setConnectionInfo((prev) => ({
          ...prev,
          state: "disconnected",
          clientId: null,
        }));

        onDisconnect?.();

        // Attempt reconnect if not manual disconnect
        if (!isManualDisconnectRef.current && event.code !== 1000) {
          scheduleReconnect();
        }
      };
    } catch (e) {
      console.error("[WebSocket] Failed to create connection:", e);
      setConnectionInfo((prev) => ({
        ...prev,
        state: "disconnected",
        lastError: e instanceof Error ? e.message : "Connection failed",
      }));
      scheduleReconnect();
    }
  }, [url, onMessage, onConnect, onDisconnect, onError, clearReconnectTimeout, scheduleReconnect]);

  // Update connectRef so scheduleReconnect can call connect
  useEffect(() => {
    connectRef.current = connect;
  }, [connect]);

  const disconnect = useCallback(() => {
    isManualDisconnectRef.current = true;
    clearReconnectTimeout();
    reconnectAttemptsRef.current = 0;

    if (wsRef.current) {
      wsRef.current.close(1000, "Manual disconnect");
      wsRef.current = null;
    }

    setConnectionInfo((prev) => ({
      ...prev,
      state: "disconnected",
      clientId: null,
      reconnectAttempts: 0,
    }));
  }, [clearReconnectTimeout]);

  const send = useCallback((message: WsClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      // Not connected
    }
  }, []);

  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      isManualDisconnectRef.current = true;
      clearReconnectTimeout();
      if (wsRef.current) {
        wsRef.current.close(1000, "Component unmount");
      }
    };
  }, [autoConnect, connect, clearReconnectTimeout]);

  return {
    connectionInfo,
    connect,
    disconnect,
    send,
    isConnected: connectionInfo.state === "connected",
  };
}
