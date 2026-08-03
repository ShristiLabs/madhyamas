import { ScrollArea } from '@/components/ui/scroll-area';
import { usePluginLogs } from '@/lib/api/tools';
import type { PluginInvocationLog } from '@/lib/api/tools';
import { CheckCircle, XCircle, AlertCircle, Clock } from 'lucide-react';

interface PluginLogsProps {
  pluginId: string;
  limit?: number;
}

export function PluginLogs({ pluginId, limit = 50 }: PluginLogsProps) {
  const { data: logs = [], isLoading } = usePluginLogs(pluginId, limit);

  if (isLoading) {
    return (
      <div className="p-3 text-xs text-muted-foreground">Loading logs...</div>
    );
  }

  if (logs.length === 0) {
    return (
      <div className="p-3 text-xs text-muted-foreground">
        No invocation logs yet. Logs appear when the plugin processes requests.
      </div>
    );
  }

  return (
    <ScrollArea className="h-full max-h-96">
      <div className="space-y-1 p-1">
        {logs.map((log) => (
          <LogEntry key={log.id} log={log} />
        ))}
      </div>
    </ScrollArea>
  );
}

function LogEntry({ log }: { log: PluginInvocationLog }) {
  const time = new Date(log.timestamp).toLocaleTimeString();

  return (
    <div
      className={`border rounded text-[10px] p-2 ${
        log.success ? 'border-green-200' : 'border-red-200'
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-1.5">
          {log.success ? (
            <CheckCircle className="w-3 h-3 text-green-500" />
          ) : (
            <XCircle className="w-3 h-3 text-red-500" />
          )}
          <span className="font-mono font-medium">{log.hook}</span>
          {log.modified && (
            <span className="px-1 py-0.5 bg-blue-100 text-blue-700 rounded text-[9px]">
              modified
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 text-muted-foreground">
          <Clock className="w-2.5 h-2.5" />
          <span>{log.duration_ms}ms</span>
          <span>·</span>
          <span>{time}</span>
        </div>
      </div>
      {log.error && (
        <div className="flex items-start gap-1 text-red-600 mb-1">
          <AlertCircle className="w-3 h-3 flex-shrink-0 mt-0.5" />
          <span className="font-mono break-all">{log.error}</span>
        </div>
      )}
      {log.logs.length > 0 && (
        <div className="bg-muted/50 rounded p-1 mt-1">
          {log.logs.map((line, i) => (
            <div key={i} className="font-mono text-[9px] text-muted-foreground break-all">
              {line}
            </div>
          ))}
        </div>
      )}
      {log.fuel_consumed != null && (
        <div className="text-[9px] text-muted-foreground mt-1">
          Fuel: {log.fuel_consumed.toLocaleString()}
        </div>
      )}
    </div>
  );
}
