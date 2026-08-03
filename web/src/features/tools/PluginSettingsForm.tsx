import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  usePluginSettings,
  usePluginSettingsSchema,
  useUpdatePluginSettings,
} from '@/lib/api/tools';
import type { PluginSettingField } from '@/lib/api/tools';
import { Save, Loader2 } from 'lucide-react';

interface PluginSettingsFormProps {
  pluginId: string;
}

export function PluginSettingsForm({ pluginId }: PluginSettingsFormProps) {
  const { data: schema, isLoading: schemaLoading } = usePluginSettingsSchema(pluginId);
  const { data: currentSettings } = usePluginSettings(pluginId);
  const updateSettings = useUpdatePluginSettings();

  const [localSettings, setLocalSettings] = useState<Record<string, unknown>>({});
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (currentSettings) {
      setLocalSettings(currentSettings);
      setDirty(false);
    }
  }, [currentSettings]);

  if (schemaLoading) {
    return (
      <div className="flex items-center justify-center p-4 text-xs text-muted-foreground">
        <Loader2 className="w-3 h-3 mr-1 animate-spin" /> Loading settings...
      </div>
    );
  }

  if (!schema || schema.fields.length === 0) {
    return (
      <div className="p-3 text-xs text-muted-foreground">
        This plugin has no configurable settings.
      </div>
    );
  }

  const handleChange = (key: string, value: unknown) => {
    setLocalSettings((prev) => ({ ...prev, [key]: value }));
    setDirty(true);
  };

  const handleSave = () => {
    updateSettings.mutate(
      { id: pluginId, settings: localSettings },
      { onSuccess: () => setDirty(false) }
    );
  };

  const handleReset = () => {
    if (currentSettings) {
      setLocalSettings(currentSettings);
      setDirty(false);
    }
  };

  return (
    <div className="space-y-3">
      <div className="space-y-2">
        {schema.fields.map((field) => (
          <SettingField
            key={field.key}
            field={field}
            value={localSettings[field.key] ?? field.default}
            onChange={(v) => handleChange(field.key, v)}
          />
        ))}
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          className="h-7 text-xs"
          onClick={handleSave}
          disabled={!dirty || updateSettings.isPending}
        >
          {updateSettings.isPending ? (
            <Loader2 className="w-3 h-3 mr-1 animate-spin" />
          ) : (
            <Save className="w-3 h-3 mr-1" />
          )}
          Save
        </Button>
        {dirty && (
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={handleReset}>
            Reset
          </Button>
        )}
      </div>
    </div>
  );
}

interface SettingFieldProps {
  field: PluginSettingField;
  value: unknown;
  onChange: (value: unknown) => void;
}

function SettingField({ field, value, onChange }: SettingFieldProps) {
  const fieldId = `setting-${field.key}`;

  const renderField = () => {
    switch (field.field_type) {
      case 'boolean':
        return (
          <input
            id={fieldId}
            type="checkbox"
            checked={Boolean(value)}
            onChange={(e) => onChange(e.target.checked)}
            className="h-4 w-4"
          />
        );
      case 'number':
        return (
          <Input
            id={fieldId}
            type="number"
            className="h-7 text-xs"
            value={value as number}
            onChange={(e) => onChange(Number(e.target.value))}
          />
        );
      case 'select':
        return (
          <select
            id={fieldId}
            className="h-7 text-xs w-full rounded border px-2"
            value={value as string}
            onChange={(e) => onChange(e.target.value)}
          >
            {field.options?.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        );
      case 'textarea':
      case 'json':
        return (
          <textarea
            id={fieldId}
            className="w-full rounded border p-2 text-xs font-mono"
            rows={4}
            value={typeof value === 'string' ? value : JSON.stringify(value, null, 2)}
            onChange={(e) => {
              if (field.field_type === 'json') {
                try {
                  onChange(JSON.parse(e.target.value));
                } catch {
                  // Keep raw string until valid JSON.
                  onChange(e.target.value);
                }
              } else {
                onChange(e.target.value);
              }
            }}
          />
        );
      default:
        return (
          <Input
            id={fieldId}
            type="text"
            className="h-7 text-xs"
            value={value as string}
            onChange={(e) => onChange(e.target.value)}
          />
        );
    }
  };

  return (
    <div className="space-y-1">
      <label htmlFor={fieldId} className="text-xs font-medium flex items-center gap-1">
        {field.label}
        {field.required && <span className="text-red-500">*</span>}
      </label>
      {field.description && (
        <p className="text-[10px] text-muted-foreground">{field.description}</p>
      )}
      {renderField()}
    </div>
  );
}
