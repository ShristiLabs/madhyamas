import { useState } from 'react';
import { Copy, Check } from 'lucide-react';

/**
 * In-script developer documentation shown on the Scripts page.
 *
 * Mirrors the runtime capabilities documented in
 * `docs/SCRIPTING.md` and `crates/madhyamas-core/src/scripting/` so that
 * script authors have everything they need without leaving the UI.
 */

interface CodeBlockProps {
  code: string;
  language?: string;
}

function CodeBlock({ code }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="relative group">
      <pre className="text-[11px] font-mono bg-muted rounded p-2 pr-8 overflow-x-auto whitespace-pre leading-relaxed">
        {code}
      </pre>
      <button
        onClick={handleCopy}
        className="absolute top-1 right-1 p-1 rounded bg-background/80 border opacity-0 group-hover:opacity-100 transition-opacity"
        title="Copy"
      >
        {copied ? (
          <Check className="w-3 h-3 text-green-500" />
        ) : (
          <Copy className="w-3 h-3" />
        )}
      </button>
    </div>
  );
}

interface SectionProps {
  title: string;
  children: React.ReactNode;
}

function Section({ title, children }: SectionProps) {
  return (
    <section className="space-y-2">
      <h3 className="text-sm font-semibold border-b pb-1">{title}</h3>
      <div className="space-y-2 text-xs leading-relaxed">{children}</div>
    </section>
  );
}

interface PropRow {
  name: string;
  type: string;
  description: string;
}

function PropTable({ rows }: { rows: PropRow[] }) {
  return (
    <table className="w-full text-[11px] border-collapse">
      <thead>
        <tr className="border-b text-left">
          <th className="py-1 pr-2 font-medium">Property</th>
          <th className="py-1 pr-2 font-medium">Type</th>
          <th className="py-1 font-medium">Description</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.name} className="border-b last:border-0">
            <td className="py-1 pr-2 font-mono text-primary whitespace-nowrap">{row.name}</td>
            <td className="py-1 pr-2 font-mono text-muted-foreground whitespace-nowrap">{row.type}</td>
            <td className="py-1 text-muted-foreground">{row.description}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const HOOKS = [
  {
    hook: 'on_request',
    fn: 'onRequest(request, context)',
    when: 'Before a request is forwarded upstream',
    canModify: 'request object',
  },
  {
    hook: 'on_response',
    fn: 'onResponse(request, response, context)',
    when: 'After a response is received from the server',
    canModify: 'response object',
  },
  {
    hook: 'on_websocket_message',
    fn: 'onWebSocketMessage(context)',
    when: 'On WebSocket message send/receive',
    canModify: 'context',
  },
  {
    hook: 'on_grpc_message',
    fn: 'onGrpcMessage(context)',
    when: 'On gRPC message send/receive',
    canModify: 'context',
  },
  {
    hook: 'on_traffic_store',
    fn: 'onTrafficStore(context)',
    when: 'When a traffic entry is persisted',
    canModify: 'context',
  },
  {
    hook: 'on_session_start',
    fn: 'onSessionStart(context)',
    when: 'When a session starts',
    canModify: 'context',
  },
  {
    hook: 'on_session_end',
    fn: 'onSessionEnd(context)',
    when: 'When a session ends',
    canModify: 'context',
  },
];

const REQUEST_PROPS: PropRow[] = [
  { name: 'method', type: 'string', description: 'HTTP method (GET, POST, PUT, ...)' },
  { name: 'url', type: 'string', description: 'Full request URL including query string' },
  { name: 'host', type: 'string', description: 'Hostname (e.g. api.example.com)' },
  { name: 'path', type: 'string', description: 'URL path without query string' },
  { name: 'headers', type: 'object', description: 'Request headers — modifiable in place' },
  { name: 'body', type: 'string | null', description: 'Request body as text (null if binary/empty)' },
  { name: 'contentType', type: 'string | null', description: 'Content-Type header value' },
  { name: 'query', type: 'object', description: 'Parsed query parameters as key/value map' },
];

const RESPONSE_PROPS: PropRow[] = [
  { name: 'statusCode', type: 'number', description: 'HTTP status code (e.g. 200, 404)' },
  { name: 'statusMessage', type: 'string | null', description: 'Status message (e.g. "OK")' },
  { name: 'headers', type: 'object', description: 'Response headers — modifiable in place' },
  { name: 'body', type: 'string | null', description: 'Response body as text (null if binary/empty)' },
  { name: 'contentType', type: 'string | null', description: 'Content-Type header value' },
  { name: 'durationMs', type: 'number', description: 'Response time in milliseconds' },
];

const CONTEXT_PROPS: PropRow[] = [
  { name: 'requestId', type: 'string', description: 'Unique ID for the current request' },
  { name: 'sessionId', type: 'string', description: 'Current session ID' },
  { name: 'hook', type: 'string', description: 'Name of the hook that fired (e.g. "on_request")' },
  { name: 'data', type: 'object', description: 'Shared mutable storage between hooks for the same request' },
];

const RETURN_PROPS: PropRow[] = [
  { name: 'continue', type: 'boolean', description: 'false = short-circuit and return `response` to the client' },
  { name: 'modified', type: 'boolean', description: 'true = proxy reads back the modified request/response object' },
  { name: 'response', type: 'object', description: 'Custom response (only when continue is false): { statusCode, headers, body }' },
];

const BUILTINS = [
  {
    name: 'console.log(...args)',
    desc: 'Log messages. Output is captured and shown in the test dialog and execution history.',
    example: 'console.log("Processing:", request.method, request.url);',
  },
  {
    name: 'JSON.parse(str)',
    desc: 'Parse a JSON string into an object/array (built into the engine).',
    example: 'var body = JSON.parse(response.body);',
  },
  {
    name: 'JSON.stringify(value)',
    desc: 'Serialise a value to a JSON string (built into the engine).',
    example: 'response.body = JSON.stringify({ ok: true });',
  },
  {
    name: 'base64.encode(str)',
    desc: 'Encode a UTF-8 string to base64.',
    example: 'var encoded = base64.encode("hello"); // "aGVsbG8="',
  },
  {
    name: 'base64.decode(str)',
    desc: 'Decode a base64 string back to UTF-8 text.',
    example: 'var decoded = base64.decode(encoded); // "hello"',
  },
  {
    name: 'crypto.hash(str)',
    desc: 'SHA-256 hex digest of the input string.',
    example: 'var digest = crypto.hash("test"); // 9f86d081...',
  },
  {
    name: 'url.parse(urlString)',
    desc: 'Parse a URL into { scheme, host, port, path, query, fragment }.',
    example: 'var parts = url.parse(request.url);',
  },
  {
    name: 'url.build(components)',
    desc: 'Build a URL string from { scheme, host, port?, path, query?, fragment? }.',
    example: 'var u = url.build({ scheme: "https", host: "api.com", path: "/v2" });',
  },
];

const EXAMPLES: { title: string; code: string }[] = [
  {
    title: 'Log every request',
    code: `function onRequest(request, context) {
    console.log(request.method + ' ' + request.url);
    return { continue: true };
}`,
  },
  {
    title: 'Add CORS headers to responses',
    code: `function onResponse(request, response, context) {
    response.headers['Access-Control-Allow-Origin'] = '*';
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, PUT, DELETE';
    response.headers['Access-Control-Allow-Headers'] = '*';
    return { continue: true, modified: true };
}`,
  },
  {
    title: 'Block specific domains',
    code: `var blocked = ['ads.example.com', 'tracker.example.com'];

function onRequest(request, context) {
    var parts = url.parse(request.url);
    if (blocked.indexOf(parts.host) !== -1) {
        console.log('Blocked: ' + parts.host);
        return {
            continue: false,
            response: { statusCode: 403, body: 'Blocked by Madhyamas' }
        };
    }
    return { continue: true };
}`,
  },
  {
    title: 'Mock an API response',
    code: `function onRequest(request, context) {
    if (request.url.indexOf('/api/user/') !== -1) {
        return {
            continue: false,
            response: {
                statusCode: 200,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ id: 123, name: 'Mock User' })
            }
        };
    }
    return { continue: true };
}`,
  },
  {
    title: 'Add a custom header to every request',
    code: `function onRequest(request, context) {
    request.headers['X-Madhyamas'] = 'true';
    request.headers['X-Request-ID'] = context.requestId;
    return { continue: true, modified: true };
}`,
  },
  {
    title: 'Pass data between request and response hooks',
    code: `function onRequest(request, context) {
    context.data.startTime = Date.now();
    return { continue: true };
}

function onResponse(request, response, context) {
    var elapsed = Date.now() - context.data.startTime;
    console.log('Request took ' + elapsed + 'ms');
    return { continue: true };
}`,
  },
  {
    title: 'Modify a JSON response body',
    code: `function onResponse(request, response, context) {
    if (response.headers['Content-Type']?.indexOf('json') !== -1) {
        var body = JSON.parse(response.body);
        body.inspectedBy = 'madhyamas';
        response.body = JSON.stringify(body);
        return { continue: true, modified: true };
    }
    return { continue: true };
}`,
  },
];

export function ScriptGuide() {
  return (
    <div className="p-3 space-y-5 max-w-2xl mx-auto">
      <header className="space-y-1">
          <h2 className="text-base font-semibold">Scripting Developer Guide</h2>
          <p className="text-xs text-muted-foreground">
            Scripts are written in JavaScript (ES6+) and executed by an embedded{' '}
            <span className="font-mono">boa_engine</span> runtime. Define a hook
            function that returns a result object — the proxy applies your
            changes to live traffic.
          </p>
        </header>

        <Section title="1. Anatomy of a script">
          <p>
            A script is a single JavaScript source file that defines one or more
            hook functions. Each hook is a top-level function whose name matches
            the event it handles (camelCase, e.g.{' '}
            <span className="font-mono">onRequest</span>). The proxy calls the
            matching function whenever that event fires.
          </p>
          <CodeBlock code={`// Minimal script: log every request
function onRequest(request, context) {
    console.log(request.method, request.url);
    return { continue: true };
}`} />
          <p className="text-muted-foreground">
            Attach a script to one or more hooks when you create it. A single
            script can define multiple hook functions (e.g. both{' '}
            <span className="font-mono">onRequest</span> and{' '}
            <span className="font-mono">onResponse</span>) to share state via{' '}
            <span className="font-mono">context.data</span>.
          </p>
        </Section>

        <Section title="2. Available hooks">
          <p>
            Each hook fires at a specific point in the proxy pipeline. The
            function signature determines which objects you receive.
          </p>
          <table className="w-full text-[11px] border-collapse">
            <thead>
              <tr className="border-b text-left">
                <th className="py-1 pr-2 font-medium">Hook</th>
                <th className="py-1 pr-2 font-medium">JS Function</th>
                <th className="py-1 pr-2 font-medium">When it fires</th>
                <th className="py-1 font-medium">Can modify</th>
              </tr>
            </thead>
            <tbody>
              {HOOKS.map((h) => (
                <tr key={h.hook} className="border-b last:border-0">
                  <td className="py-1 pr-2 font-mono text-primary whitespace-nowrap">{h.hook}</td>
                  <td className="py-1 pr-2 font-mono whitespace-nowrap">{h.fn}</td>
                  <td className="py-1 pr-2 text-muted-foreground">{h.when}</td>
                  <td className="py-1 font-mono text-muted-foreground whitespace-nowrap">{h.canModify}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>

        <Section title="3. Match filter (restricting which requests fire the script)">
          <p>
            By default, a script fires on <strong>every</strong> request that
            matches its hook. To restrict it to specific requests, set a{' '}
            <strong>match filter</strong> in the script editor (the collapsible{' '}
            "Match Filter" panel). The proxy checks the filter{' '}
            <em>before</em> invoking the JS engine, so non-matching requests
            have zero overhead.
          </p>
          <p>The filter has four optional fields — all must match for the script to fire:</p>
          <PropTable
            rows={[
              { name: 'method', type: 'string', description: 'HTTP method (exact, case-insensitive). E.g. "GET"' },
              { name: 'host_pattern', type: 'glob', description: 'Glob matched against the request host. E.g. "*.example.com"' },
              { name: 'path_pattern', type: 'glob', description: 'Glob matched against the URL path. E.g. "/api/v2/*"' },
              { name: 'url_pattern', type: 'glob', description: 'Glob matched against the full URL. E.g. "*/api/users*"' },
            ]}
          />
          <p>
            Glob patterns support <span className="font-mono">*</span> (any
            sequence) and <span className="font-mono">?</span> (single
            character). Matching is case-insensitive. Leave all fields empty to
            match every request (the default).
          </p>
          <CodeBlock code={`// Example: only fire on GET requests to *.example.com/api/*
// Set in the Match Filter panel:
//   method:        GET
//   host_pattern:  *.example.com
//   path_pattern:  /api/*

// The script body can then assume it only runs on matching requests:
function onRequest(request, context) {
    console.log('API call:', request.url);
    return { continue: true };
}`} />
          <p className="text-muted-foreground">
            You can also filter inside the script body with{' '}
            <span className="font-mono">if</span> conditions — but the
            declarative match filter is more efficient because it skips the JS
            engine entirely for non-matching requests.
          </p>
        </Section>

        <Section title="4. The request object">
          <p>
            Passed to <span className="font-mono">onRequest</span> and{' '}
            <span className="font-mono">onResponse</span>. Mutate{' '}
            <span className="font-mono">headers</span>,{' '}
            <span className="font-mono">body</span>, etc. in place and return{' '}
            <span className="font-mono">{`{ continue: true, modified: true }`}</span> to
            apply the changes.
          </p>
          <PropTable rows={REQUEST_PROPS} />
        </Section>

        <Section title="5. The response object">
          <p>
            Passed to <span className="font-mono">onResponse</span>. Mutate in
            place and return <span className="font-mono">{`{ modified: true }`}</span>.
          </p>
          <PropTable rows={RESPONSE_PROPS} />
        </Section>

        <Section title="6. The context object">
          <p>
            Passed to every hook. Use{' '}
            <span className="font-mono">context.data</span> to share mutable
            state between the <span className="font-mono">onRequest</span> and{' '}
            <span className="font-mono">onResponse</span> hooks of the same
            request.
          </p>
          <PropTable rows={CONTEXT_PROPS} />
        </Section>

        <Section title="7. Return value">
          <p>
            Every hook function must return an object with these fields:
          </p>
          <CodeBlock code={`return {
    continue: true,      // false = short-circuit, return custom response
    modified: false,     // true = proxy reads back the modified object
    response: {          // only when continue is false
        statusCode: 403,
        headers: { 'Content-Type': 'text/plain' },
        body: 'Blocked'
    }
};`} />
          <PropTable rows={RETURN_PROPS} />
        </Section>

        <Section title="8. Built-in libraries & objects">
          <p>
            The runtime is sandboxed (no filesystem, network, or process
            access). These globals are available in every script:
          </p>
          <div className="space-y-2">
            {BUILTINS.map((b) => (
              <div key={b.name} className="border rounded p-2 space-y-1">
                <div className="font-mono text-primary text-[11px]">{b.name}</div>
                <div className="text-muted-foreground">{b.desc}</div>
                <CodeBlock code={b.example} />
              </div>
            ))}
          </div>
          <p className="text-muted-foreground">
            Standard JavaScript built-ins such as{' '}
            <span className="font-mono">Math</span>,{' '}
            <span className="font-mono">Date</span>,{' '}
            <span className="font-mono">Array</span>,{' '}
            <span className="font-mono">Object</span>,{' '}
            <span className="font-mono">String</span>,{' '}
            <span className="font-mono">Number</span>,{' '}
            <span className="font-mono">RegExp</span>, and{' '}
            <span className="font-mono">parseInt</span>/
            <span className="font-mono">parseFloat</span> are also available via
            the boa engine.
          </p>
        </Section>

        <Section title="9. Examples">
          <div className="space-y-3">
            {EXAMPLES.map((ex) => (
              <div key={ex.title} className="space-y-1">
                <div className="text-xs font-medium">{ex.title}</div>
                <CodeBlock code={ex.code} />
              </div>
            ))}
          </div>
        </Section>

        <Section title="10. Security & limits">
          <ul className="list-disc pl-4 space-y-1 text-muted-foreground">
            <li>
              Scripts run in a fresh <span className="font-mono">boa_engine</span>{' '}
              context per execution — no shared state between scripts.
            </li>
            <li>
              No filesystem, network, or process access is exposed. The runtime
              is sandboxed by construction.
            </li>
            <li>
              Execution time is soft-limited by{' '}
              <span className="font-mono">timeout_ms</span> (default 5000ms).
              Long-running scripts are flagged as timed out.
            </li>
            <li>
              Scripts are trusted code — created by the proxy operator. Do not
              run untrusted scripts.
            </li>
            <li>
              Scripts persist to SQLite (<span className="font-mono">~/.madhyamas/traffic.db</span>)
              and survive restarts.
            </li>
          </ul>
        </Section>

        <footer className="text-[10px] text-muted-foreground pt-2 border-t">
          See <span className="font-mono">docs/SCRIPTING.md</span>,{' '}
          <span className="font-mono">docs/SCRIPTING_API.md</span>, and{' '}
          <span className="font-mono">docs/SCRIPTING_SECURITY.md</span> for the
          full reference.
        </footer>
    </div>
  );
}
