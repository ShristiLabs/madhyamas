import { useCallback } from "react";
import { Search, AlertCircle, Clock, Globe, X } from "lucide-react";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { AddFilterPopover, FilterChip } from "./FilterBuilder";
import type { ActiveFilter } from "@/types/filters";

interface TrafficToolbarProps {
  search: string;
  onSearchChange: (search: string) => void;
  filters: ActiveFilter[];
  onFiltersChange: (filters: ActiveFilter[]) => void;
  count?: number;
}

interface QuickFilterDef {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  activeFilter: ActiveFilter;
  description: string;
}

const QUICK_FILTERS: QuickFilterDef[] = [
  {
    id: "qf-errors",
    label: "Errors",
    icon: AlertCircle,
    activeFilter: {
      id: "qf-errors",
      fieldId: "status_category",
      operator: "eq",
      value: "4xx",
    },
    description: "4xx Client Errors",
  },
  {
    id: "qf-slow",
    label: "Slow",
    icon: Clock,
    activeFilter: {
      id: "qf-slow",
      fieldId: "duration",
      operator: "gt",
      value: "1000",
    },
    description: "Requests > 1s",
  },
  {
    id: "qf-api",
    label: "API",
    icon: Globe,
    activeFilter: {
      id: "qf-api",
      fieldId: "path",
      operator: "contains",
      value: "/api/",
    },
    description: "API endpoints",
  },
];

export function TrafficToolbar({
  search,
  onSearchChange,
  filters,
  onFiltersChange,
  count,
}: TrafficToolbarProps) {
  const handleQuickFilter = useCallback(
    (qf: QuickFilterDef) => {
      const isActive = filters.some((f) => f.id === qf.id);
      if (isActive) {
        onFiltersChange(filters.filter((f) => f.id !== qf.id));
      } else {
        onFiltersChange([...filters, { ...qf.activeFilter }]);
      }
    },
    [filters, onFiltersChange],
  );

  const handleAddFilter = useCallback(
    (filter: ActiveFilter) => {
      onFiltersChange([...filters, filter]);
    },
    [filters, onFiltersChange],
  );

  const handleRemoveFilter = useCallback(
    (id: string) => {
      onFiltersChange(filters.filter((f) => f.id !== id));
    },
    [filters, onFiltersChange],
  );

  const handleClearAll = useCallback(() => {
    onFiltersChange([]);
    onSearchChange("");
  }, [onFiltersChange, onSearchChange]);

  const hasActiveFilters = filters.length > 0 || search.length > 0;

  return (
    <div className="border-b">
      {/* Main toolbar row */}
      <div className="px-4 py-2.5 flex items-center gap-2.5 flex-wrap">
        {/* Search */}
        <div className="relative flex-1 max-w-sm min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search traffic..."
            className="pl-9 h-9"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            aria-label="Search traffic"
          />
        </div>

        {/* Add Filter */}
        <AddFilterPopover onAdd={handleAddFilter} />

        {/* Quick Filters */}
        <div className="flex items-center gap-1">
          {QUICK_FILTERS.map((qf) => {
            const isActive = filters.some((f) => f.id === qf.id);
            const Icon = qf.icon;
            return (
              <Button
                key={qf.id}
                variant={isActive ? "default" : "outline"}
                size="sm"
                onClick={() => handleQuickFilter(qf)}
                className="h-8"
                title={qf.description}
              >
                <Icon className="h-3.5 w-3.5 mr-1" />
                {qf.label}
              </Button>
            );
          })}
        </div>

        {/* Clear all + Count */}
        <div className="flex items-center gap-2 ml-auto">
          {hasActiveFilters && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClearAll}
              className="h-8 text-muted-foreground"
            >
              <X className="h-3.5 w-3.5 mr-1" />
              Clear{filters.length > 0 && ` (${filters.length})`}
            </Button>
          )}
          {count !== undefined && (
            <span className="text-sm text-muted-foreground">
              {count} requests
            </span>
          )}
        </div>
      </div>

      {/* Filter chips row */}
      {filters.length > 0 && (
        <div className="px-4 pb-2.5 flex items-center gap-1.5 flex-wrap">
          {filters.map((f) => (
            <FilterChip key={f.id} filter={f} onRemove={handleRemoveFilter} />
          ))}
        </div>
      )}
    </div>
  );
}
