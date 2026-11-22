(function() {
    let _hidden = false;
    let _pauseStyleEl = null;

    // The page is hidden when the panel says so (hidden to the tray, minimized: `__setWebviewVisibility`) or when the
    // engine says so (the window is covered by other windows). The engine's own answer is read before the property
    // is replaced; a frame of another origin is told only by the engine.
    let _engineHidden = function() { return false; };
    try {
        const descriptor = Object.getOwnPropertyDescriptor(Document.prototype, 'visibilityState');
        if (descriptor && descriptor.get) {
            _engineHidden = function() {
                try { return descriptor.get.call(document) === 'hidden'; } catch (e) { return false; }
            };
        }
    } catch (e) {}
    const isHidden = function() { return _hidden || _engineHidden(); };

    Object.defineProperty(document, 'hidden', {
        get: function() { return isHidden(); },
        configurable: true,
    });
    Object.defineProperty(document, 'visibilityState', {
        get: function() { return isHidden() ? 'hidden' : 'visible'; },
        configurable: true,
    });

    function pauseAnimations() {
        if (!_pauseStyleEl) {
            _pauseStyleEl = document.createElement('style');
            _pauseStyleEl.id = '__gpu_pause_animations';
            _pauseStyleEl.textContent = '*, *::before, *::after { animation-play-state: paused !important; transition: none !important; }';
        }
        if (!_pauseStyleEl.parentNode) {
            (document.head || document.documentElement).appendChild(_pauseStyleEl);
        }
    }

    function resumeAnimations() {
        if (_pauseStyleEl && _pauseStyleEl.parentNode) {
            _pauseStyleEl.parentNode.removeChild(_pauseStyleEl);
        }
    }

    // What was applied last: the animations follow the combined state, not one of its two sources.
    let _applied = false;
    function apply() {
        const now = isHidden();
        if (now === _applied) return false;
        _applied = now;
        console.log('[WebView Visibility] ' + (now ? '进入休眠' : '恢复活跃'));
        if (now) {
            pauseAnimations();
        } else {
            resumeAnimations();
        }
        return true;
    }

    window.__setWebviewVisibility = function(visible) {
        _hidden = !visible;
        // The page's own listeners get the event only when the combined state changed.
        if (apply()) document.dispatchEvent(new Event('visibilitychange'));
    };

    // The engine's own events (a covered window, a frame): the animations follow them; the page's listeners
    // receive these events themselves and read the combined state.
    if (typeof document.addEventListener === 'function') {
        document.addEventListener('visibilitychange', apply);
    }

    // WebView 心跳：每 10 秒向 Rust 后端发送心跳信号
    // 若渲染进程崩溃，心跳停止，后端检测到后可触发自动恢复
    // Only the page of the window beats: the Web UI frame (another origin) is not allowed to call the command, and every beat of it was an unhandled rejection in its console and in the log
    // (`Command webview_heartbeat not allowed by ACL`, every 10 s). A rejection of a beat is swallowed in any case.
    var beat = function() {
        try {
            if (window.__TAURI_INTERNALS__ && window.top === window) {
                var answer = window.__TAURI_INTERNALS__.invoke('webview_heartbeat', {generation: __LIFECYCLE_GENERATION__});
                if (answer && typeof answer.catch === 'function') answer.catch(function() {});
            }
        } catch(e) {}
    };
    setInterval(beat, 10000);
    // 立即发送首次心跳
    beat();
})();
