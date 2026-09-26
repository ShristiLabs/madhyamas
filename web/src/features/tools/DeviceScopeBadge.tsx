import { Smartphone } from "lucide-react";

/**
 * Device-scope indicator for intercept rules (enterprise, issue #109).
 *
 * Renders nothing for global rules (`device_id` absent/null), so the OSS
 * tier — where the API never attaches a device scope — sees no device
 * column anywhere. For device-scoped rules it shows a short device ID
 * with the full ID in a tooltip.
 */
export function DeviceScopeBadge({ deviceId }: { deviceId?: string | null }) {
  if (!deviceId) return null;
  const short = deviceId.length > 10 ? `${deviceId.slice(0, 10)}…` : deviceId;
  return (
    <span
      title={`Device-scoped rule: ${deviceId}`}
      className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300 flex-shrink-0"
    >
      <Smartphone className="h-3 w-3" />
      {short}
    </span>
  );
}
