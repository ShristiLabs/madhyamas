//! Lazy-loaded Ace-based code editor for the scripting panel.
//!
//! This component is code-split into its own chunk so the Ace editor
//! (~600 KB) only loads when the user opens the Scripts panel and starts
//! editing a script.  The rest of the app stays lightweight.
//!
//! Features: JavaScript syntax highlighting, autocomplete, snippets,
//! bracket matching, code folding, find/replace, line numbers, soft tabs,
//! and a dark theme that blends with the app's dark UI.

import { useEffect, useRef, useCallback } from 'react';
import AceEditor from 'react-ace';

// Ace modes and themes are side-effect imports that register themselves
// on the Ace singleton.  We import the JavaScript mode and a dark theme.
// `ace-builds` ships pre-built ESM modules that work with Vite's bundler.
import 'ace-builds/src-noconflict/mode-javascript';
import 'ace-builds/src-noconflict/theme-tomorrow_night_blue';
import 'ace-builds/src-noconflict/ext-language_tools';
import 'ace-builds/src-noconflict/ext-searchbox';
// Worker — enables syntax validation / error annotations for JS.
import 'ace-builds/src-noconflict/worker-javascript';

interface ScriptCodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  /** Read-only mode (for display-only contexts). */
  readOnly?: boolean;
}

/**
 * Full-featured Ace code editor for JavaScript scripts.
 *
 * Configured with:
 * - JavaScript mode with syntax highlighting
 * - `tomorrow_night_blue` theme (dark, matches the app's dark UI)
 * - Autocomplete + snippets (language tools ext)
 * - Find/replace (searchbox ext)
 * - Code folding, bracket matching, line numbers
 * - Soft tabs (2 spaces), 80-column print margin
 */
export function ScriptCodeEditor({
  value,
  onChange,
  readOnly,
}: ScriptCodeEditorProps) {
  // react-ace's default export is a class component; use a loose ref
  // type to avoid friction with the library's internal typing.
  const editorRef = useRef<{ editor: { focus: () => void } } | null>(null);

  // Focus the editor on mount for immediate editing.
  useEffect(() => {
    editorRef.current?.editor.focus();
  }, []);

  const handleChange = useCallback(
    (val: string) => onChange(val),
    [onChange],
  );

  return (
    <AceEditor
      ref={editorRef as never}
      mode="javascript"
      theme="tomorrow_night_blue"
      name="script-code-editor"
      value={value}
      onChange={handleChange}
      readOnly={readOnly}
      width="100%"
      height="100%"
      fontSize={13}
      tabSize={2}
      showPrintMargin={true}
      showGutter={true}
      highlightActiveLine={true}
      enableBasicAutocompletion={true}
      enableLiveAutocompletion={true}
      enableSnippets={true}
      wrapEnabled={true}
      setOptions={{
        useWorker: true,
        showLineNumbers: true,
        showGutter: true,
        fontFamily:
          'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
        fontSize: '13px',
        tabSize: 2,
        useSoftTabs: true,
        wrap: true,
        indentedSoftWrap: true,
        foldStyle: 'markbegin',
        behavioursEnabled: true,
        wrapBehavioursEnabled: true,
        autoScrollEditorIntoView: true,
        copyWithEmptySelection: true,
        highlightSelectedWord: true,
      }}
      editorProps={{ $blockScrolling: true }}
      style={{ width: '100%', height: '100%' }}
    />
  );
}

export default ScriptCodeEditor;
