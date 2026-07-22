import { useState, useRef, useEffect, useCallback } from "react";
import { Plus, X, Search, ChevronRight, ChevronLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  type ActiveFilter,
  type FilterFieldDef,
  type FilterOperator,
  FILTER_FIELDS,
  OPERATOR_LABELS,
} from "@/types/filters";

interface AddFilterPopoverProps {
  onAdd: (filter: ActiveFilter) => void;
}

let nextFilterId = 1;

export function AddFilterPopover({ onAdd }: AddFilterPopoverProps) {
  const [open, setOpen] = useState(false);
  const [step, setStep] = useState<"field" | "configure">("field");
  const [selectedField, setSelectedField] = useState<FilterFieldDef | null>(
    null,
  );
  const [operator, setOperator] = useState<FilterOperator>("contains");
  const [value, setValue] = useState("");
  const [key, setKey] = useState("");
  const [fieldSearch, setFieldSearch] = useState("");
  const popoverRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const resetAndClose = useCallback(() => {
    setOpen(false);
    setStep("field");
    setSelectedField(null);
    setOperator("contains");
    setValue("");
    setKey("");
    setFieldSearch("");
  }, []);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        resetAndClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, resetAndClose]);

  const handleFieldSelect = useCallback((field: FilterFieldDef) => {
    setSelectedField(field);
    setOperator(field.operators[0]);
    setValue(field.options?.[0]?.value ?? "");
    setKey("");
    setStep("configure");
  }, []);

  const handleAdd = useCallback(() => {
    if (!selectedField) return;
    if (operator !== "exists" && operator !== "not_exists" && !value) return;

    const newFilter: ActiveFilter = {
      id: `filter-${nextFilterId++}`,
      fieldId: selectedField.id,
      operator,
      value: operator === "exists" || operator === "not_exists" ? "" : value,
      ...(selectedField.hasKey && key ? { key } : {}),
    };

    onAdd(newFilter);
    resetAndClose();
  }, [selectedField, operator, value, key, onAdd, resetAndClose]);

  const filteredFields = FILTER_FIELDS.filter((f) =>
    f.label.toLowerCase().includes(fieldSearch.toLowerCase()),
  );
  const requestFields = filteredFields.filter((f) => f.category === "request");
  const responseFields = filteredFields.filter(
    (f) => f.category === "response",
  );

  const needsValue = operator !== "exists" && operator !== "not_exists";

  return (
    <div className="relative">
      <Button
        ref={buttonRef}
        variant="outline"
        size="sm"
        className="h-8"
        onClick={() => {
          if (open) resetAndClose();
          else setOpen(true);
        }}
      >
        <Plus className="h-3.5 w-3.5 mr-1" />
        Add Filter
      </Button>

      {open && (
        <div
          ref={popoverRef}
          className="absolute top-full left-0 mt-1 z-50 bg-popover border rounded-lg shadow-lg w-80"
        >
          {step === "field" ? (
            <div>
              <div className="p-2 border-b">
                <div className="relative">
                  <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
                  <Input
                    placeholder="Search fields..."
                    className="h-8 pl-8 text-sm"
                    value={fieldSearch}
                    onChange={(e) => setFieldSearch(e.target.value)}
                    autoFocus
                  />
                </div>
              </div>
              <div className="max-h-64 overflow-y-auto p-1">
                {requestFields.length > 0 && (
                  <>
                    <div className="px-2 py-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                      Request
                    </div>
                    {requestFields.map((f) => (
                      <button
                        key={f.id}
                        className="w-full flex items-center justify-between px-2 py-1.5 text-sm rounded hover:bg-muted cursor-pointer"
                        onClick={() => handleFieldSelect(f)}
                      >
                        <span>{f.label}</span>
                        <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                      </button>
                    ))}
                  </>
                )}
                {responseFields.length > 0 && (
                  <>
                    <div className="px-2 py-1 mt-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                      Response
                    </div>
                    {responseFields.map((f) => (
                      <button
                        key={f.id}
                        className="w-full flex items-center justify-between px-2 py-1.5 text-sm rounded hover:bg-muted cursor-pointer"
                        onClick={() => handleFieldSelect(f)}
                      >
                        <span>{f.label}</span>
                        <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
                      </button>
                    ))}
                  </>
                )}
                {filteredFields.length === 0 && (
                  <div className="px-2 py-4 text-sm text-muted-foreground text-center">
                    No matching fields
                  </div>
                )}
              </div>
            </div>
          ) : selectedField ? (
            <div>
              <div className="px-3 py-2 border-b flex items-center gap-2">
                <button
                  className="text-muted-foreground hover:text-foreground"
                  onClick={() => {
                    setStep("field");
                    setSelectedField(null);
                  }}
                >
                  <ChevronLeft className="h-4 w-4" />
                </button>
                <span className="text-sm font-medium">
                  {selectedField.label}
                </span>
              </div>
              <div className="p-3 space-y-3">
                {selectedField.hasKey && (
                  <div>
                    <label className="text-xs text-muted-foreground mb-1 block">
                      {selectedField.id === "cookie"
                        ? "Cookie Name"
                        : "Header Name"}
                    </label>
                    <Input
                      className="h-8 text-sm"
                      placeholder={
                        selectedField.id === "cookie"
                          ? "session_id"
                          : "Authorization"
                      }
                      value={key}
                      onChange={(e) => setKey(e.target.value)}
                      autoFocus
                    />
                  </div>
                )}

                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">
                    Operator
                  </label>
                  <div className="flex flex-wrap gap-1">
                    {selectedField.operators.map((op) => (
                      <button
                        key={op}
                        className={cn(
                          "px-2 py-1 text-xs rounded border transition-colors",
                          operator === op
                            ? "bg-primary text-primary-foreground border-primary"
                            : "bg-background hover:bg-muted border-border",
                        )}
                        onClick={() => setOperator(op)}
                      >
                        {OPERATOR_LABELS[op]}
                      </button>
                    ))}
                  </div>
                </div>

                {needsValue && (
                  <div>
                    <label className="text-xs text-muted-foreground mb-1 block">
                      Value
                    </label>
                    {selectedField.valueType === "select" &&
                    selectedField.options ? (
                      <div className="flex flex-wrap gap-1 max-h-32 overflow-y-auto">
                        {selectedField.options.map((opt) => (
                          <button
                            key={opt.value}
                            className={cn(
                              "px-2 py-1 text-xs rounded border transition-colors",
                              value === opt.value
                                ? "bg-primary text-primary-foreground border-primary"
                                : "bg-background hover:bg-muted border-border",
                            )}
                            onClick={() => setValue(opt.value)}
                          >
                            {opt.label}
                          </button>
                        ))}
                      </div>
                    ) : (
                      <Input
                        className="h-8 text-sm"
                        type={
                          selectedField.valueType === "number"
                            ? "number"
                            : "text"
                        }
                        placeholder={selectedField.placeholder}
                        value={value}
                        onChange={(e) => setValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") handleAdd();
                        }}
                        autoFocus={!selectedField.hasKey}
                      />
                    )}
                  </div>
                )}

                <Button size="sm" className="w-full h-8" onClick={handleAdd}>
                  Add Filter
                </Button>
              </div>
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}

interface FilterChipProps {
  filter: ActiveFilter;
  onRemove: (id: string) => void;
}

export function FilterChip({ filter, onRemove }: FilterChipProps) {
  const field = FILTER_FIELDS.find((f) => f.id === filter.fieldId);
  if (!field) return null;

  const opLabel = OPERATOR_LABELS[filter.operator];
  let displayValue = filter.value;
  if (field.options) {
    const opt = field.options.find((o) => o.value === filter.value);
    if (opt) displayValue = opt.label;
  }

  return (
    <div className="inline-flex items-center gap-1 h-7 px-2.5 rounded-full bg-primary/10 border border-primary/20 text-xs select-none">
      <span className="font-medium text-foreground">
        {field.label}
        {filter.key && (
          <span className="text-muted-foreground">: {filter.key}</span>
        )}
      </span>
      <span className="text-muted-foreground">{opLabel}</span>
      {filter.operator !== "exists" && filter.operator !== "not_exists" && (
        <span className="font-mono text-foreground">{displayValue}</span>
      )}
      <button
        className="ml-0.5 rounded-full p-0.5 hover:bg-destructive/20 hover:text-destructive transition-colors"
        onClick={() => onRemove(filter.id)}
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
