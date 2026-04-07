/**
 * Tests for the Life Savor Config Page Template SDK.
 *
 * Uses a lightweight mock WebSocket to verify message flow, correlation ID
 * matching, timeout handling, and error propagation without a real server.
 */

const { LifeSavorConfigSDK, generateCorrelationId } = require('./config-sdk');

// ---------------------------------------------------------------------------
// Mock WebSocket
// ---------------------------------------------------------------------------

class MockWebSocket {
  constructor(url) {
    this.url = url;
    this.readyState = 0; // CONNECTING
    this.sent = [];
    this.onopen = null;
    this.onclose = null;
    this.onerror = null;
    this.onmessage = null;

    // Auto-open on next tick so connect() resolves
    MockWebSocket._lastInstance = this;
  }

  simulateOpen() {
    this.readyState = 1; // OPEN
    if (this.onopen) this.onopen({});
  }

  simulateMessage(data) {
    if (this.onmessage) this.onmessage({ data: typeof data === 'string' ? data : JSON.stringify(data) });
  }

  simulateClose() {
    this.readyState = 3; // CLOSED
    if (this.onclose) this.onclose({});
  }

  simulateError() {
    if (this.onerror) this.onerror({});
  }

  send(data) {
    if (this.readyState !== 1) throw new Error('WebSocket is not open');
    this.sent.push(JSON.parse(data));
  }

  close() {
    this.readyState = 3;
    if (this.onclose) this.onclose({});
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function createSDK(overrides) {
  var sdk = new LifeSavorConfigSDK({
    wsUrl: 'wss://test.example.com/ws',
    componentId: 'ollama-llm',
    timeout: 500,
    ...overrides,
  });
  // Inject mock WebSocket constructor
  sdk._WebSocket = MockWebSocket;
  return sdk;
}

async function connectSDK(sdk) {
  var p = sdk.connect();
  MockWebSocket._lastInstance.simulateOpen();
  await p;
  return MockWebSocket._lastInstance;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('LifeSavorConfigSDK', () => {
  // -- Constructor validation -----------------------------------------------

  test('throws when wsUrl is missing', () => {
    expect(() => new LifeSavorConfigSDK({ componentId: 'x' })).toThrow('wsUrl is required');
  });

  test('throws when componentId is missing', () => {
    expect(() => new LifeSavorConfigSDK({ wsUrl: 'wss://x' })).toThrow('componentId is required');
  });

  // -- Connection -----------------------------------------------------------

  test('connect() resolves when WebSocket opens', async () => {
    var sdk = createSDK();
    var p = sdk.connect();
    MockWebSocket._lastInstance.simulateOpen();
    await expect(p).resolves.toBeUndefined();
  });

  test('connect() rejects when WebSocket errors before open', async () => {
    var sdk = createSDK();
    var p = sdk.connect();
    MockWebSocket._lastInstance.simulateError();
    await expect(p).rejects.toThrow('connection failed');
  });

  test('connect() is idempotent when already connected', async () => {
    var sdk = createSDK();
    await connectSDK(sdk);
    // Second connect should resolve immediately
    await expect(sdk.connect()).resolves.toBeUndefined();
  });

  test('disconnect() closes the WebSocket', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);
    sdk.disconnect();
    expect(ws.readyState).toBe(3);
  });

  // -- getConfig() ----------------------------------------------------------

  test('getConfig() sends correct message and resolves with config', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();

    // Verify sent message
    expect(ws.sent).toHaveLength(1);
    var msg = ws.sent[0];
    expect(msg.type).toBe('component.config.get');
    expect(msg.componentId).toBe('ollama-llm');
    expect(msg.correlationId).toBeDefined();

    // Simulate response
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      config: { temperature: 0.7 },
    });

    var result = await p;
    expect(result).toEqual({ temperature: 0.7 });
  });

  // -- updateConfig() -------------------------------------------------------

  test('updateConfig() sends config payload and resolves', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var newConfig = { temperature: 0.9, max_tokens: 2048 };
    var p = sdk.updateConfig(newConfig);

    var msg = ws.sent[0];
    expect(msg.type).toBe('component.config.update');
    expect(msg.componentId).toBe('ollama-llm');
    expect(msg.config).toEqual(newConfig);

    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      config: newConfig,
    });

    var result = await p;
    expect(result).toEqual(newConfig);
  });

  test('updateConfig() rejects when config is null', async () => {
    var sdk = createSDK();
    await connectSDK(sdk);
    await expect(sdk.updateConfig(null)).rejects.toThrow('config is required');
  });

  test('updateConfig() rejects when config is undefined', async () => {
    var sdk = createSDK();
    await connectSDK(sdk);
    await expect(sdk.updateConfig(undefined)).rejects.toThrow('config is required');
  });

  // -- getSchema() ----------------------------------------------------------

  test('getSchema() sends correct message and resolves with schema', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getSchema();

    var msg = ws.sent[0];
    expect(msg.type).toBe('component.config.schema');
    expect(msg.componentId).toBe('ollama-llm');

    var schema = { type: 'object', properties: { temperature: { type: 'number' } } };
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      schema: schema,
    });

    var result = await p;
    expect(result).toEqual(schema);
  });

  // -- Error handling -------------------------------------------------------

  test('rejects with error message from response', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();
    var msg = ws.sent[0];

    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      error: 'component not found',
    });

    await expect(p).rejects.toThrow('component not found');
  });

  test('rejects on timeout', async () => {
    var sdk = createSDK({ timeout: 50 });
    await connectSDK(sdk);

    await expect(sdk.getConfig()).rejects.toThrow('timed out');
  });

  test('rejects when not connected', async () => {
    var sdk = createSDK();
    await expect(sdk.getConfig()).rejects.toThrow('not connected');
  });

  test('rejects pending requests on connection close', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();
    ws.simulateClose();

    await expect(p).rejects.toThrow('connection closed');
  });

  // -- Correlation ID matching ----------------------------------------------

  test('matches responses to correct pending requests', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p1 = sdk.getConfig();
    var p2 = sdk.getSchema();

    var msg1 = ws.sent[0];
    var msg2 = ws.sent[1];

    // Respond to second request first
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg2.correlationId,
      schema: { type: 'object' },
    });

    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg1.correlationId,
      config: { temp: 1 },
    });

    var r1 = await p1;
    var r2 = await p2;

    expect(r1).toEqual({ temp: 1 });
    expect(r2).toEqual({ type: 'object' });
  });

  test('ignores messages with unknown correlationId', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();
    var msg = ws.sent[0];

    // Unknown correlation ID — should be ignored
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: 'unknown-id',
      config: { bad: true },
    });

    // Correct response
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      config: { good: true },
    });

    var result = await p;
    expect(result).toEqual({ good: true });
  });

  test('ignores non-config-response messages', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();
    var msg = ws.sent[0];

    // Different message type — should be ignored
    ws.simulateMessage({
      type: 'savo.response',
      correlationId: msg.correlationId,
      data: 'hello',
    });

    // Correct response
    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      config: { ok: true },
    });

    var result = await p;
    expect(result).toEqual({ ok: true });
  });

  test('ignores non-JSON messages', async () => {
    var sdk = createSDK();
    var ws = await connectSDK(sdk);

    var p = sdk.getConfig();
    var msg = ws.sent[0];

    // Non-JSON — should be silently ignored
    ws.simulateMessage('not json at all');

    ws.simulateMessage({
      type: 'component.config.response',
      correlationId: msg.correlationId,
      config: { ok: true },
    });

    var result = await p;
    expect(result).toEqual({ ok: true });
  });
});

// ---------------------------------------------------------------------------
// generateCorrelationId
// ---------------------------------------------------------------------------

describe('generateCorrelationId', () => {
  test('returns a string in UUID-like format', () => {
    var id = generateCorrelationId();
    expect(typeof id).toBe('string');
    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  });

  test('generates unique IDs', () => {
    var ids = new Set();
    for (var i = 0; i < 100; i++) {
      ids.add(generateCorrelationId());
    }
    expect(ids.size).toBe(100);
  });
});
