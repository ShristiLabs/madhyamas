import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import {
  useGrpcConnections,
  useGrpcStreams,
  useGrpcFrames,
  useGrpcStats,
  useClearGrpcFrames
} from '@/lib/api/phase3';
import type { GrpcConnection, GrpcStream, GrpcFrame, GrpcStats, GrpcFilter } from '@/lib/api/phase3';
import { Trash2, Search, Activity, Network, Layers, BarChart3 } from 'lucide-react';

export function GrpcPanel() {
  const [activeTab, setActiveTab] = useState('frames');
  const [filter, setFilter] = useState<GrpcFilter>({});

  const { data: connections = [] } = useGrpcConnections();
  const { data: streams = [] } = useGrpcStreams();
  const { data: frames = [] } = useGrpcFrames(filter);
  const { data: stats } = useGrpcStats();
  const clearFrames = useClearGrpcFrames();

  const tabs = [
    { value: 'frames', label: 'Frames', icon: <Layers className="w-4 h-4" /> },
    { value: 'connections', label: 'Connections', icon: <Network className="w-4 h-4" /> },
    { value: 'streams', label: 'Streams', icon: <Activity className="w-4 h-4" /> },
    { value: 'stats', label: 'Stats', icon: <BarChart3 className="w-4 h-4" /> },
  ];

  const handleClearFrames = () => {
    clearFrames.mutate();
  };

  const filteredFrames = frames.filter(f => {
    if (filter.service && !f.service?.toLowerCase().includes(filter.service.toLowerCase())) return false;
    if (filter.method && !f.method?.toLowerCase().includes(filter.method.toLowerCase())) return false;
    if (filter.search && !f.payload?.toLowerCase().includes(filter.search.toLowerCase())) return false;
    return true;
  });

  return (
    <div className="h-full flex flex-col">
      <div className="p-2 border-b space-y-2">
        <div className="flex items-center gap-2">
          <div className="flex-1 relative">
            <Search className="w-3 h-3 absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Search frames..."
              className="h-7 text-xs pl-6"
              value={filter.search || ''}
              onChange={(e) => setFilter({ ...filter, search: e.target.value })}
            />
          </div>
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={handleClearFrames}
          >
            <Trash2 className="w-3 h-3 mr-1" />
            Clear
          </Button>
        </div>
        <div className="flex gap-2">
          <Input
            placeholder="Service"
            className="h-7 text-xs flex-1"
            value={filter.service || ''}
            onChange={(e) => setFilter({ ...filter, service: e.target.value })}
          />
          <Input
            placeholder="Method"
            className="h-7 text-xs flex-1"
            value={filter.method || ''}
            onChange={(e) => setFilter({ ...filter, method: e.target.value })}
          />
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <TabsList className="grid grid-cols-4 h-9 p-0 m-2">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.value} value={tab.value} className="text-xs py-1">
              {tab.icon}
            </TabsTrigger>
          ))}
        </TabsList>

        <ScrollArea className="flex-1">
          <TabsContent value="connections" className="m-0 p-2">
            {connections.length === 0 ? (
              <div className="text-xs text-muted-foreground text-center py-4">
                No gRPC connections
              </div>
            ) : (
              <div className="space-y-1">
                {connections.map((conn) => (
                  <GrpcConnectionItem key={conn.id} connection={conn} />
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="streams" className="m-0 p-2">
            {streams.length === 0 ? (
              <div className="text-xs text-muted-foreground text-center py-4">
                No gRPC streams
              </div>
            ) : (
              <div className="space-y-1">
                {streams.map((stream) => (
                  <GrpcStreamItem key={stream.id} stream={stream} />
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="frames" className="m-0 p-2">
            {filteredFrames.length === 0 ? (
              <div className="text-xs text-muted-foreground text-center py-4">
                No gRPC frames
              </div>
            ) : (
              <div className="space-y-1">
                {filteredFrames.map((frame) => (
                  <GrpcFrameItem key={frame.id} frame={frame} />
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="stats" className="m-0 p-2">
            {stats && <GrpcStatsView stats={stats} />}
          </TabsContent>
        </ScrollArea>
      </Tabs>
    </div>
  );
}

function GrpcConnectionItem({ connection }: { connection: GrpcConnection }) {
  return (
    <div className="border rounded p-2 text-xs">
      <div className="flex items-center justify-between">
        <span className="font-mono">{connection.host}:{connection.port}</span>
        <span className={`px-1.5 py-0.5 rounded text-[10px] ${
          connection.state === 'active' ? 'bg-green-100 text-green-700' :
          connection.state === 'idle' ? 'bg-yellow-100 text-yellow-700' :
          'bg-gray-100 text-gray-700'
        }`}>
          {connection.state}
        </span>
      </div>
      <div className="text-muted-foreground mt-1">
        ID: {connection.id.slice(0, 8)}...
      </div>
    </div>
  );
}

function GrpcStreamItem({ stream }: { stream: GrpcStream }) {
  return (
    <div className="border rounded p-2 text-xs">
      <div className="flex items-center justify-between">
        <span className="font-medium">{stream.service}/{stream.method}</span>
        <span className={`px-1.5 py-0.5 rounded text-[10px] ${
          stream.state === 'open' ? 'bg-green-100 text-green-700' :
          stream.state === 'idle' ? 'bg-yellow-100 text-yellow-700' :
          'bg-gray-100 text-gray-700'
        }`}>
          {stream.state}
        </span>
      </div>
      <div className="text-muted-foreground mt-1 flex gap-2">
        <span>{stream.direction}</span>
        <span>•</span>
        <span>{stream.message_count} msgs</span>
      </div>
    </div>
  );
}

function GrpcFrameItem({ frame }: { frame: GrpcFrame }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border rounded p-2 text-xs">
      <div
        className="flex items-center justify-between cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <span className={`px-1.5 py-0.5 rounded text-[10px] ${
            frame.direction === 'request' ? 'bg-blue-100 text-blue-700' :
            'bg-purple-100 text-purple-700'
          }`}>
            {frame.direction}
          </span>
          <span className="font-mono text-[10px]">{frame.service}/{frame.method}</span>
        </div>
        <span className="text-muted-foreground">#{frame.sequence}</span>
      </div>
      {expanded && (
        <div className="mt-2 p-2 bg-muted rounded text-[10px] font-mono overflow-auto max-h-32">
          {frame.payload || '<empty>'}
        </div>
      )}
    </div>
  );
}

function GrpcStatsView({ stats }: { stats: GrpcStats }) {
  const statItems = [
    { label: 'Total Connections', value: stats.total_connections },
    { label: 'Active Connections', value: stats.active_connections },
    { label: 'Total Streams', value: stats.total_streams },
    { label: 'Active Streams', value: stats.active_streams },
    { label: 'Total Frames', value: stats.total_frames },
    { label: 'Frames Sent', value: stats.frames_sent },
    { label: 'Frames Received', value: stats.frames_received },
  ];

  return (
    <div className="space-y-2">
      {statItems.map((item) => (
        <div key={item.label} className="flex justify-between items-center text-xs">
          <span className="text-muted-foreground">{item.label}</span>
          <span className="font-mono">{item.value}</span>
        </div>
      ))}
    </div>
  );
}
