/**
 * Life Savor Config Page Template SDK
 *
 * Lightweight JavaScript SDK for config page templates that run inside
 * sandboxed iframes on the marketplace CDN. Communicates with the agent
 * bridge via WebSocket through the connect service.
 *
 * Message types:
 *   → component.config.get      – fetch current config
 *   → component.config.update   – persist new config
 *   → component.config.schema   – fetch JSON Schema for the config
 *   ← component.config.response – response from agent bridge
 *
 * Usage:
 *   const sdk = new LifeSavorConfigSDK({ wsUrl, componentId });
 *   await sdk.connect();
 *   const config = await sdk.getConfig();
 *   await sdk.updateConfig({ temperature: 0.8 });
 *   const schema = await sdk.getSchema();
 *   sdk.disconnect();
 */

(function (root) {
  'use strict';

  var DEFAULT_TIMEOUT_MS = 10000;

  /**
   * Generate a random correlation ID (v4-ish UUID without crypto dependency).
   */
  function generateCorrelationId() {
    var chars = 'abcdef0123456789';
    var sections = [8, 4, 4, 4, 12];
    var parts = [];
    for (var s = 0; s < sections.length; s++) {
      var part = '';
      for (var i = 0; i < sections[s]; i++) {
        part += chars[Math.floor(Math.random() * chars.length)];
      }
      parts.push(part);
    }
    return parts.join('-');
  }

  /**
   * @constructor
   * @param {Object} options
   * @param {string} options.wsUrl        – WebSocket endpoint (connect service)
   * @param {string} options.componentId  – target component ID
   * @param {number} [options.timeout]    – request timeout in ms (default 10 000)
   */
  function LifeSavorConfigSDK(options) {
    if (!options || !options.wsUrl) {
      throw new Error('LifeSavorConfigSDK: wsUrl is required');
    }
    if (!options.componentId) {
      throw new Error('LifeSavorConfigSDK: componentId is required');
    }

    this._wsUrl = options.wsUrl;
    this._componentId = options.componentId;
    this._timeout = options.timeout || DEFAULT_TIMEOUT_MS;
    this._ws = null;
    this._pending = {}; // correlationId → { resolve, reject, timer }
    this._connected = false;
  }

  /**
   * Open the WebSocket connection. Resolves when the socket is open.
   * @returns {Promise<void>}
   */
  LifeSavorConfigSDK.prototype.connect = function () {
    var self = this;

    if (self._connected && self._ws && self._ws.readyState === 1) {
      return Promise.resolve();
    }

    return new Promise(function (resolve, reject) {
      try {
        self._ws = new (self._WebSocket || root.WebSocket)(self._wsUrl);
      } catch (err) {
        return reject(new Error('LifeSavorConfigSDK: failed to create WebSocket – ' + err.message));
      }

      self._ws.onopen = function () {
        self._connected = true;
        resolve();
      };

      self._ws.onerror = function (evt) {
        if (!self._connected) {
          reject(new Error('LifeSavorConfigSDK: WebSocket connection failed'));
        }
      };

      self._ws.onclose = function () {
        self._connected = false;
        // Reject all pending requests
        var ids = Object.keys(self._pending);
        for (var i = 0; i < ids.length; i++) {
          var entry = self._pending[ids[i]];
          clearTimeout(entry.timer);
          entry.reject(new Error('LifeSavorConfigSDK: connection closed'));
        }
        self._pending = {};
      };

      self._ws.onmessage = function (evt) {
        self._handleMessage(evt.data);
      };
    });
  };

  /**
   * Close the WebSocket connection.
   */
  LifeSavorConfigSDK.prototype.disconnect = function () {
    if (this._ws) {
      this._ws.close();
      this._ws = null;
    }
    this._connected = false;
  };

  /**
   * Fetch the current config from the component.
   * @returns {Promise<Object>} – the config object
   */
  LifeSavorConfigSDK.prototype.getConfig = function () {
    return this._sendRequest('component.config.get');
  };

  /**
   * Persist updated config to the component.
   * @param {Object} config – the new config payload
   * @returns {Promise<Object>} – acknowledgement / updated config
   */
  LifeSavorConfigSDK.prototype.updateConfig = function (config) {
    if (config === undefined || config === null) {
      return Promise.reject(new Error('LifeSavorConfigSDK: config is required'));
    }
    return this._sendRequest('component.config.update', { config: config });
  };

  /**
   * Fetch the JSON Schema describing the component's config.
   * @returns {Promise<Object>} – the JSON Schema object
   */
  LifeSavorConfigSDK.prototype.getSchema = function () {
    return this._sendRequest('component.config.schema');
  };

  // -----------------------------------------------------------------------
  // Internal helpers
  // -----------------------------------------------------------------------

  /**
   * Send a typed message and return a Promise that resolves with the response.
   * @private
   */
  LifeSavorConfigSDK.prototype._sendRequest = function (type, extra) {
    var self = this;

    if (!self._connected || !self._ws || self._ws.readyState !== 1) {
      return Promise.reject(new Error('LifeSavorConfigSDK: not connected'));
    }

    var correlationId = generateCorrelationId();

    var message = {
      type: type,
      componentId: self._componentId,
      correlationId: correlationId,
    };

    if (extra) {
      var keys = Object.keys(extra);
      for (var i = 0; i < keys.length; i++) {
        message[keys[i]] = extra[keys[i]];
      }
    }

    return new Promise(function (resolve, reject) {
      var timer = setTimeout(function () {
        delete self._pending[correlationId];
        reject(new Error('LifeSavorConfigSDK: request timed out (' + self._timeout + 'ms)'));
      }, self._timeout);

      self._pending[correlationId] = {
        resolve: resolve,
        reject: reject,
        timer: timer,
      };

      try {
        self._ws.send(JSON.stringify(message));
      } catch (err) {
        clearTimeout(timer);
        delete self._pending[correlationId];
        reject(new Error('LifeSavorConfigSDK: send failed – ' + err.message));
      }
    });
  };

  /**
   * Handle an incoming WebSocket message. Matches by correlationId.
   * @private
   */
  LifeSavorConfigSDK.prototype._handleMessage = function (raw) {
    var msg;
    try {
      msg = JSON.parse(raw);
    } catch (_) {
      return; // ignore non-JSON frames
    }

    if (msg.type !== 'component.config.response') {
      return; // not for us
    }

    var entry = this._pending[msg.correlationId];
    if (!entry) {
      return; // no matching pending request
    }

    clearTimeout(entry.timer);
    delete this._pending[msg.correlationId];

    if (msg.error) {
      entry.reject(new Error('LifeSavorConfigSDK: ' + msg.error));
    } else {
      entry.resolve(msg.config || msg.schema || msg);
    }
  };

  // -----------------------------------------------------------------------
  // Export
  // -----------------------------------------------------------------------

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { LifeSavorConfigSDK: LifeSavorConfigSDK, generateCorrelationId: generateCorrelationId };
  } else {
    root.LifeSavorConfigSDK = LifeSavorConfigSDK;
  }
})(typeof globalThis !== 'undefined' ? globalThis : typeof self !== 'undefined' ? self : this);
