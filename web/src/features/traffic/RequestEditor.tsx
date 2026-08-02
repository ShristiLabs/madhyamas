import { useState, useMemo, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Plus, Trash2, Pencil } from 'lucide-react';
import type { RequestModifications } from '@/lib/api/intercept';

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;

const CONTENT_TYPES = [
  { label: 'application/json', value: 'application/json' },
  { label: 'application/x-www-form-urlencoded', value: 'application/x-www-form-urlencoded' },
  { label: 'text/plain', value: 'text/plain' },
  { label: 'text/html', value: 'text/html' },
  { label: 'application/xml', value: 'application/xml' },
  { label: 'multipart/form-data', value: 'multipart/form-data' },
];

export interface RequestEditorInitialRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

export interface RequestEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialRequest: RequestEditorInitialRequest;
  onSubmit: (modifications: RequestModifications) => void;
  title?: string;
  description?: string;
}

interface HeaderRow {
  name: string;
  value: string;
}

function headersToRows(headers: Record<string, string>): HeaderRow[] {
  return Object.entries(headers).map(([name, value]) => ({ name, value }));
}

function rowsToHeaders(rows: HeaderRow[]): Record<string, string> {
  const result: Record<string, string> = {};
  for (const row of rows) {
    const name = row.name.trim();
    if (name) {
      result[name] = row.value;
    }
  }
  return result;
}

function detectContentType(headers: Record<string, string>): string | undefined {
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === 'content-type') {
      return value.split(';')[0].trim();
    }
  }
  return undefined;
}

function diffModifications(
  original: RequestEditorInitialRequest,
  edited: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body: string;
  },
): RequestModifications {
  const mods: RequestModifications = {};

  if (edited.url !== original.url) {
    mods.url = edited.url;
  }

  if (edited.method !== original.method) {
    mods.method = edited.method;
  }

  const originalHeaders = original.headers;
  const editedHeaders = edited.headers;

  const headers: Record<string, string> = {};
  const removeHeaders: string[] = [];

  for (const [name, value] of Object.entries(editedHeaders)) {
    const originalValue = originalHeaders[name];
    if (originalValue === undefined) {
      headers[name] = value;
    } else if (originalValue !== value) {
      headers[name] = value;
    }
  }

  for (const name of Object.keys(originalHeaders)) {
    if (!(name in editedHeaders)) {
      removeHeaders.push(name);
    }
  }

  if (Object.keys(headers).length > 0) {
    mods.headers = headers;
  }
  if (removeHeaders.length > 0) {
    mods.remove_headers = removeHeaders;
  }

  const originalBody = original.body ?? '';
  if (edited.body !== originalBody) {
    mods.body = edited.body;
  }

  return mods;
}

export function RequestEditor({
  open,
  onOpenChange,
  initialRequest,
  onSubmit,
  title = 'Edit Request',
  description = 'Modify the request before replaying. Only changed fields are sent.',
}: RequestEditorProps) {
  const [method, setMethod] = useState(initialRequest.method);
  const [url, setUrl] = useState(initialRequest.url);
  const [headerRows, setHeaderRows] = useState<HeaderRow[]>(() =>
    headersToRows(initialRequest.headers),
  );
  const [body, setBody] = useState(initialRequest.body ?? '');

  const detectedContentType = useMemo(
    () => detectContentType(rowsToHeaders(headerRows)),
    [headerRows],
  );

  const handleAddHeader = useCallback(() => {
    setHeaderRows((prev) => [...prev, { name: '', value: '' }]);
  }, []);

  const handleRemoveHeader = useCallback((index: number) => {
    setHeaderRows((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleHeaderChange = useCallback(
    (index: number, field: 'name' | 'value', value: string) => {
      setHeaderRows((prev) =>
        prev.map((row, i) => (i === index ? { ...row, [field]: value } : row)),
      );
    },
    [],
  );

  const handleContentTypeChange = useCallback(
    (contentType: string) => {
      setHeaderRows((prev) => {
        const existingIndex = prev.findIndex(
          (row) => row.name.toLowerCase() === 'content-type',
        );
        if (existingIndex >= 0) {
          return prev.map((row, i) =>
            i === existingIndex ? { ...row, value: contentType } : row,
          );
        }
        return [...prev, { name: 'Content-Type', value: contentType }];
      });
    },
    [],
  );

  const handleSubmit = useCallback(() => {
    const editedHeaders = rowsToHeaders(headerRows);
    const modifications = diffModifications(initialRequest, {
      method,
      url,
      headers: editedHeaders,
      body,
    });
    onSubmit(modifications);
  }, [initialRequest, headerRows, method, url, body, onSubmit]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[85vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Pencil className="h-4 w-4" />
            {title}
          </DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        <ScrollArea className="max-h-[55vh]">
          <div className="grid gap-4 py-2 pr-4">
            <div className="grid grid-cols-[120px_1fr] gap-2">
              <div className="grid gap-1.5">
                <Label>Method</Label>
                <Select value={method} onValueChange={setMethod}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {HTTP_METHODS.map((m) => (
                      <SelectItem key={m} value={m}>
                        {m}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="grid gap-1.5">
                <Label>URL</Label>
                <Input
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  className="font-mono"
                  placeholder="https://example.com/path"
                />
              </div>
            </div>

            <div className="grid gap-1.5">
              <div className="flex items-center justify-between">
                <Label>Headers</Label>
                <Button variant="ghost" size="icon-sm" onClick={handleAddHeader}>
                  <Plus className="h-3.5 w-3.5" />
                </Button>
              </div>
              <div className="space-y-1.5">
                {headerRows.length === 0 && (
                  <div className="text-xs text-muted-foreground py-2">
                    No headers. Click + to add one.
                  </div>
                )}
                {headerRows.map((row, index) => (
                  <div key={index} className="flex items-center gap-1.5">
                    <Input
                      value={row.name}
                      onChange={(e) => handleHeaderChange(index, 'name', e.target.value)}
                      placeholder="Header name"
                      className="font-mono flex-1"
                    />
                    <Input
                      value={row.value}
                      onChange={(e) => handleHeaderChange(index, 'value', e.target.value)}
                      placeholder="Header value"
                      className="font-mono flex-1"
                    />
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="text-destructive"
                      onClick={() => handleRemoveHeader(index)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>

            <div className="grid gap-1.5">
              <Label>Content-Type</Label>
              <Select
                value={detectedContentType ?? ''}
                onValueChange={handleContentTypeChange}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select content type" />
                </SelectTrigger>
                <SelectContent>
                  {CONTENT_TYPES.map((ct) => (
                    <SelectItem key={ct.value} value={ct.value}>
                      {ct.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-1.5">
              <Label>Body</Label>
              <Textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                className="font-mono min-h-[160px] resize-y"
                placeholder="Request body (leave empty for no body)"
              />
            </div>
          </div>
        </ScrollArea>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit}>Replay with Changes</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
