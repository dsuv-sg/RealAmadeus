/*
 * l2d-compare deterministic page shim (V1 bundle page and V2 viewer page load the SAME file).
 * It must be the FIRST classic (non-async, non-module) script of the page, before any Live2D code.
 *
 * Query parameters (shared by both pages):
 *   frames=N       stop after N drawn frames (default 1), then document.title = "DONE"
 *   seed=S         mulberry32 seed for Math.random (default 20261118)
 *   timeout=MS     real-time limit for the model to start drawing (default 60000); afterwards, or when
 *                  frames stall for 15 s, document.title = "TIMEOUT"
 *   readback=0     skip the RGBA canvas readback of the final frame (default on)
 *   stepms=D[,D...]  virtual frame durations in ms instead of the constant 1000/60: frame k (k >= 2) runs
 *                  D[(k-2) mod n] after frame k-1; "D*R" repeats D R times. Exercises the player's 0.1 s delta
 *                  cap, multi-step physics and its > 5 s reset. Example: stepms=16.667*59,6000,8.333,120
 *   do=K:method:json[;K:method:json...]
 *                  scripted player calls: at the start of drawn frame K (K >= 2, before that frame's
 *                  callbacks) call api[method](...args), args = the JSON value (an array is spread), api =
 *                  window.__l2dCompareApi (V1 page: the ReamaCubismPlayer) or window.l2dViewer (V2 viewer).
 *                  Both expose setEmotion / setLipSync / setSpokenChar / setEyeTracking. Frame-1 state is
 *                  set by the pages' own emotion / lip / char / eyeX / eyeY parameters (frame 1 is only known
 *                  once it has drawn). Example: do=120:setEmotion:"NORMAL";150:setEyeTracking:[0.5,-0.3]
 *
 * Virtual clock: frozen while the model loads (performance.now() = P0, Date.now() = T0, rAF timestamp = P0).
 * The first animation frame in which WebGL draws anything is frame 1 and still runs at the frozen time
 * (both V1 clocks then see dt = 0); frame k runs at P0 + (k - 1) * 1000/60 ms, or at P0 + the sum of the
 * first k - 1 stepms durations (Date.now() is that value floored to an integer ms, like the real one).
 * Every frozen loading frame is a no-op for the SDK and for the player (all their state advances by dt),
 * so V1 and V2 start frame 1 from the same state even if they need a different number of frames to load
 * (V1 also waits for its 13 shader fetches). This harness measures rendering parity, not load timing.
 *
 * requestAnimationFrame is virtualized: callbacks are queued and run, in registration order, from ONE real
 * rAF per frame with the virtual timestamp (callbacks registered during a frame run in the next one; cancel
 * works, an exception in one callback is reported and does not skip the others). After frame N no callback
 * runs any more, so the last frame stays on screen for the screenshot.
 *
 * window.__l2dCompare exposes the state; report() returns a JSON-safe summary (draw calls per frame,
 * Math.random log, contexts, canvases, console lines, errors); readback holds the PNG data URL of the WebGL
 * canvas taken right after frame N's callbacks (the drawing buffer is still intact at that point).
 */
(function () {
  'use strict';
  if (window.__l2dCompare) return;

  var q = new URLSearchParams(location.search);
  function intParam(name, def) {
    var v = parseInt(q.get(name) || '', 10);
    return isFinite(v) ? v : def;
  }
  var FRAMES = Math.max(1, intParam('frames', 1));
  var SEED = intParam('seed', 20261118) >>> 0;
  var TIMEOUT_MS = Math.max(1000, intParam('timeout', 60000));
  var STALL_MS = 15000;
  var READBACK = q.get('readback') !== '0';
  var ACTIONS = [];
  var actionError = null;
  try {
    ACTIONS = parseActions(q.get('do') || '');
  } catch (e) {
    actionError = String(e);
  }
  var STEP_MS = 1000 / 60;
  var STEPS = null; // null = constant STEP_MS (offset = (k - 1) * STEP_MS, exactly as before stepms existed)
  try {
    STEPS = parseSteps(q.get('stepms') || '');
  } catch (e) {
    actionError = (actionError ? actionError + '; ' : '') + String(e);
  }
  var T0 = 1763000000000; // Date.now() of the frozen clock (2025-11-13T02:13:20Z)
  var P0 = 10000; // performance.now() of the frozen clock; non-zero: V1 tests `!this._lastUpdateTime`
  var MAX_LOG = 400;

  var realRAF = window.requestAnimationFrame.bind(window);
  var realSetTimeout = window.setTimeout.bind(window);
  var realPerfNow = performance.now.bind(performance);
  var realStart = realPerfNow();

  function parseSteps(spec) {
    if (!spec) return null;
    var out = [];
    spec.split(',').forEach(function (item) {
      var m = /^\s*([0-9]*\.?[0-9]+)(?:\*(\d+))?\s*$/.exec(item);
      if (!m) throw new Error('l2d-compare shim: bad stepms item ' + item);
      for (var i = 0, n = m[2] ? parseInt(m[2], 10) : 1; i < n; i++) out.push(parseFloat(m[1]));
    });
    if (!out.length || out.length > 100000) throw new Error('l2d-compare shim: bad stepms length');
    return out;
  }

  function parseActions(spec) {
    var out = [];
    spec.split(';').forEach(function (item) {
      item = item.trim();
      if (!item) return;
      var m = /^(\d+):([A-Za-z_$][\w$]*):(.*)$/.exec(item);
      if (!m) throw new Error('l2d-compare shim: bad do= item ' + item);
      var frame = parseInt(m[1], 10);
      if (frame < 2) throw new Error('l2d-compare shim: do= frames start at 2 (frame 1 = page parameters)');
      var args = m[3] === '' ? [] : JSON.parse(m[3]);
      out.push({ frame: frame, method: m[2], args: Array.isArray(args) ? args : [args] });
    });
    return out;
  }

  var S = {
    version: 2,
    params: {
      frames: FRAMES,
      seed: SEED,
      timeout: TIMEOUT_MS,
      readback: READBACK,
      actions: ACTIONS,
      run: q.get('run') || '',
    },
    actionLog: [], // [drawn frame, method, args, "ok" | error]
    clock: { T0: T0, P0: P0, stepMs: STEP_MS, steps: STEPS ? STEPS.length : 0 },
    status: 'loading', // loading -> drawing -> done | never-drew | stalled
    done: false,
    armed: false,
    frame: 0, // drawn frames completed (frame 1 = first frame with WebGL draw calls)
    ticks: 0, // real animation frames that ran callbacks
    armedAtTick: 0,
    callbacksRun: 0,
    callbackErrors: 0,
    drawsOutsideFrames: 0,
    loadingTickDraws: [], // draw-call count of each tick before arming (first 200)
    perFrameDraws: [], // draw-call count of each drawn frame
    firstFrameDrawLog: [], // per draw call: "m<count>" = into an FBO (mask), "s<count>" = default framebuffer, "a<count>" = drawArrays
    lastFrameDrawLog: [],
    randomCalls: 0,
    randomCallsAtArm: 0,
    randomLog: [], // first 64 calls, see Math.random below
    perfNowCalls: 0,
    dateNowCalls: 0,
    contexts: [],
    canvases: [],
    console: [],
    errors: [],
    readback: null,
    readbackFrame: 0,
    readbackError: null,
    realMs: { armed: null, done: null },
  };
  window.__l2dCompare = S;

  // ---- virtual clock --------------------------------------------------------------------------------
  var step = 0; // virtual 1/60 s steps elapsed (frame k runs at step k - 1)
  var offset = 0; // with stepms: virtual ms since frame 1 (sum of the first `step` durations)
  function vOffset() {
    return STEPS ? offset : step * STEP_MS;
  }
  function vPerf() {
    return P0 + vOffset();
  }
  // Patched on the prototype (and before any glue loads), so both property lookups and wasm-bindgen
  // glue that caches `Performance.prototype.now` see the virtual clock.
  Object.defineProperty(Performance.prototype, 'now', {
    configurable: true,
    writable: true,
    value: function now() {
      S.perfNowCalls++;
      return vPerf();
    },
  });
  Date.now = function now() {
    S.dateNowCalls++;
    return Math.floor(T0 + vOffset());
  };
  S.now = vPerf;

  // ---- seeded Math.random (mulberry32) --------------------------------------------------------------
  var seedState = SEED | 0;
  function mulberry32() {
    seedState = (seedState + 0x6d2b79f5) | 0;
    var t = Math.imul(seedState ^ (seedState >>> 15), 1 | seedState);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  Math.random = function random() {
    var v = mulberry32();
    S.randomCalls++;
    // [tick (1-based; the running one inside a frame), inside a frame ? 1 : 0, value];
    // drawn frame = tick - armedAtTick + 1
    if (S.randomLog.length < 64) S.randomLog.push([S.ticks + (inFrame ? 1 : 0), inFrame ? 1 : 0, v]);
    return v;
  };

  // ---- virtual requestAnimationFrame ----------------------------------------------------------------
  var queue = new Map();
  var running = null;
  var nextId = 1;
  var scheduled = false;
  var inFrame = false;
  var frameDraws = 0;
  var frameLog = null;
  var lastProgress = realPerfNow();

  function schedule() {
    if (!scheduled && !S.done) {
      scheduled = true;
      realRAF(onFrame);
    }
  }
  window.requestAnimationFrame = function requestAnimationFrame(cb) {
    if (typeof cb !== 'function') {
      throw new TypeError(
        "Failed to execute 'requestAnimationFrame' on 'Window': parameter 1 is not of type 'Function'.",
      );
    }
    var id = nextId++;
    queue.set(id, cb);
    schedule();
    return id;
  };
  window.cancelAnimationFrame = function cancelAnimationFrame(id) {
    queue.delete(id);
    if (running) running.delete(id);
  };

  function report(err) {
    S.callbackErrors++;
    if (typeof window.reportError === 'function') window.reportError(err);
    else
      realSetTimeout(function () {
        throw err;
      }, 0);
  }

  function runActions(frame) {
    ACTIONS.forEach(function (a) {
      if (a.frame !== frame) return;
      var api = window.__l2dCompareApi || window.l2dViewer;
      var res = 'ok';
      try {
        if (!api || typeof api[a.method] !== 'function') throw new Error('page API has no ' + a.method + '()');
        api[a.method].apply(api, a.args);
      } catch (e) {
        res = String(e);
        if (S.errors.length < MAX_LOG) S.errors.push('action ' + a.method + ' @frame ' + frame + ': ' + res);
      }
      S.actionLog.push([frame, a.method, a.args, res]);
    });
  }

  function onFrame() {
    scheduled = false;
    if (S.done) return;
    var batch = queue;
    queue = new Map();
    if (S.armed) {
      S.frame += 1;
      step = S.frame - 1;
      if (STEPS && step > 0) offset += STEPS[(step - 1) % STEPS.length];
      runActions(S.frame);
    }
    var ts = vPerf();
    running = batch;
    inFrame = true;
    frameDraws = 0;
    frameLog = [];
    batch.forEach(function (cb, id) {
      if (!batch.has(id)) return; // cancelled by an earlier callback of this frame
      batch.delete(id);
      S.callbacksRun++;
      try {
        cb(ts);
      } catch (e) {
        report(e);
      }
    });
    inFrame = false;
    running = null;
    S.ticks++;
    if (!S.armed) {
      if (S.loadingTickDraws.length < 200) S.loadingTickDraws.push(frameDraws);
      if (frameDraws > 0) {
        S.armed = true;
        S.status = 'drawing';
        S.frame = 1;
        S.armedAtTick = S.ticks;
        S.randomCallsAtArm = S.randomCalls;
        S.realMs.armed = Math.round(realPerfNow() - realStart);
      }
    }
    if (S.armed) {
      S.perFrameDraws.push(frameDraws);
      if (S.frame === 1) S.firstFrameDrawLog = frameLog;
      S.lastFrameDrawLog = frameLog;
      lastProgress = realPerfNow();
      if (S.frame >= FRAMES) {
        finish('DONE', true);
        return;
      }
    }
    if (queue.size) schedule();
  }

  // ---- WebGL instrumentation (draw calls, framebuffer target, contexts) ------------------------------
  var fboBound = false;
  function noteDraw(tag) {
    if (inFrame) {
      frameDraws++;
      if (frameLog.length < 2000) frameLog.push(tag);
    } else {
      S.drawsOutsideFrames++;
    }
  }
  function wrap(proto, name, make) {
    var orig = proto && proto[name];
    if (typeof orig === 'function') proto[name] = make(orig);
  }
  function hookGL(proto) {
    if (!proto) return;
    wrap(proto, 'drawElements', function (orig) {
      return function drawElements(mode, count) {
        noteDraw((fboBound ? 'm' : 's') + count);
        return orig.apply(this, arguments);
      };
    });
    wrap(proto, 'drawArrays', function (orig) {
      return function drawArrays(mode, first, count) {
        noteDraw('a' + count);
        return orig.apply(this, arguments);
      };
    });
    wrap(proto, 'bindFramebuffer', function (orig) {
      return function bindFramebuffer(target, fb) {
        if (target !== 0x8ca8 /* READ_FRAMEBUFFER */) fboBound = !!fb;
        return orig.apply(this, arguments);
      };
    });
  }
  hookGL(window.WebGLRenderingContext && window.WebGLRenderingContext.prototype);
  hookGL(window.WebGL2RenderingContext && window.WebGL2RenderingContext.prototype);

  var glCanvases = [];
  var origGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function getContext(type) {
    var ctx = origGetContext.apply(this, arguments);
    if (ctx && /webgl/i.test(String(type)) && glCanvases.indexOf(this) < 0) {
      glCanvases.push(this);
      var requested = null;
      try {
        requested = arguments[1] ? JSON.parse(JSON.stringify(arguments[1])) : null;
      } catch (e) {
        requested = String(e);
      }
      var actual = null;
      try {
        actual = ctx.getContextAttributes();
      } catch (e) {
        actual = String(e);
      }
      S.contexts.push({ type: String(type), requested: requested, actual: actual });
    }
    return ctx;
  };

  function describeCanvases() {
    return Array.prototype.map.call(document.querySelectorAll('canvas'), function (c) {
      var r = c.getBoundingClientRect();
      return {
        width: c.width,
        height: c.height,
        clientWidth: c.clientWidth,
        clientHeight: c.clientHeight,
        rect: [r.left, r.top, r.width, r.height],
        style: c.getAttribute('style') || '',
        webgl: glCanvases.indexOf(c) >= 0,
      };
    });
  }

  // ---- console / error capture (messages are also forwarded unchanged) -------------------------------
  function fmt(v) {
    try {
      if (typeof v === 'string') return v;
      if (v instanceof Error) return v.stack || String(v);
      if (v && typeof v === 'object') {
        var s = JSON.stringify(v);
        return s && s.length > 300 ? s.slice(0, 300) + '…' : String(s);
      }
      return String(v);
    } catch (e) {
      return Object.prototype.toString.call(v);
    }
  }
  ['log', 'info', 'warn', 'error', 'debug'].forEach(function (level) {
    var orig = console[level];
    if (typeof orig !== 'function') return;
    console[level] = function () {
      if (S.console.length < MAX_LOG) {
        S.console.push(level + ': ' + Array.prototype.map.call(arguments, fmt).join(' '));
      }
      return orig.apply(console, arguments);
    };
  });
  window.addEventListener(
    'error',
    function (e) {
      if (S.errors.length >= MAX_LOG) return;
      if (e.target && e.target !== window) {
        S.errors.push('resource error: ' + (e.target.src || e.target.href || e.target.tagName));
      } else {
        S.errors.push(
          'error: ' + (e.error ? fmt(e.error) : e.message) + (e.filename ? ' @' + e.filename + ':' + e.lineno : ''),
        );
      }
    },
    true,
  );
  window.addEventListener('unhandledrejection', function (e) {
    if (S.errors.length < MAX_LOG) S.errors.push('unhandledrejection: ' + fmt(e.reason));
  });

  // ---- finish ---------------------------------------------------------------------------------------
  function finish(title, insideFrame) {
    if (S.done) return;
    S.done = true;
    S.status = title === 'DONE' ? 'done' : S.armed ? 'stalled' : 'never-drew';
    S.realMs.done = Math.round(realPerfNow() - realStart);
    try {
      S.canvases = describeCanvases();
    } catch (e) {
      S.canvases = [String(e)];
    }
    if (READBACK) {
      var c = glCanvases.filter(function (x) {
        return x.isConnected;
      })[0];
      if (!c) S.readbackError = 'no WebGL canvas';
      else if (!insideFrame) S.readbackError = 'not taken inside a frame (drawing buffer already presented)';
      else {
        try {
          S.readback = c.toDataURL('image/png');
          S.readbackFrame = S.frame;
        } catch (e) {
          S.readbackError = String(e);
        }
      }
    }
    document.title = title;
  }

  // Resolves after two more REAL animation frames (no virtual callback runs any more once done), i.e. once
  // the compositor has presented the last drawn frame: capture.mjs awaits it before the screenshot.
  S.presented = function () {
    return new Promise(function (resolve) {
      realRAF(function () {
        realRAF(function () {
          resolve(true);
        });
      });
    });
  };

  S.report = function () {
    var out = {};
    Object.keys(S).forEach(function (k) {
      if (k !== 'readback' && typeof S[k] !== 'function') out[k] = S[k];
    });
    out.hasReadback = !!S.readback;
    out.canvasesNow = describeCanvases();
    return out;
  };

  if (actionError) {
    S.errors.push('bad query: ' + actionError);
    realSetTimeout(function () {
      finish('TIMEOUT', false);
    }, 0);
  }

  function watchdog() {
    if (S.done) return;
    var now = realPerfNow();
    if (!S.armed && now - realStart > TIMEOUT_MS) finish('TIMEOUT', false);
    else if (S.armed && now - lastProgress > STALL_MS) finish('TIMEOUT', false);
    else realSetTimeout(watchdog, 500);
  }
  realSetTimeout(watchdog, 500);
})();
