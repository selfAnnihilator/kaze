/* Opt-in test instrumentation, injected by the performance harness before React.
   Never imported by the application or included in its production bundle. */
(() => {
  if (window.__kazePerf) return;
  const nativeRAF = window.requestAnimationFrame.bind(window);
  const nativeTimeout = window.setTimeout.bind(window);
  const counters = {
    commits: 0,
    renders: {},
    layouts: 0,
    resizeDeliveries: 0,
    raf: 0,
    timer: 0,
    mutations: {},
    ipc: {},
  };
  const pendingRAF = new Set(),
    timers = new Set(),
    intervals = new Set();
  const globals = new Map();
  const trustedInput = [];
  for (const type of ["click", "keydown", "wheel"])
    document.addEventListener(
      type,
      (event) => {
        if (event.isTrusted) {
          trustedInput.push({
            type,
            key: event.key,
            text: event.target?.textContent?.slice(0, 50),
          });
          if (trustedInput.length > 30) trustedInput.shift();
        }
      },
      true,
    );
  const add = EventTarget.prototype.addEventListener;
  const remove = EventTarget.prototype.removeEventListener;
  EventTarget.prototype.addEventListener = function (type, listener, options) {
    if ((this === window || this === document) && listener) {
      const key = `${this === window ? "window" : "document"}:${type}:${Boolean(typeof options === "boolean" ? options : options?.capture)}`;
      if (!globals.has(key)) globals.set(key, new Set());
      globals.get(key).add(listener);
    }
    return add.call(this, type, listener, options);
  };
  EventTarget.prototype.removeEventListener = function (
    type,
    listener,
    options,
  ) {
    const key = `${this === window ? "window" : "document"}:${type}:${Boolean(typeof options === "boolean" ? options : options?.capture)}`;
    globals.get(key)?.delete(listener);
    return remove.call(this, type, listener, options);
  };
  const observerTargets = {
    ResizeObserver: new Map(),
    IntersectionObserver: new Map(),
  };
  for (const name of Object.keys(observerTargets)) {
    const Original = window[name];
    if (!Original) continue;
    window[name] = class extends Original {
      constructor(callback, options) {
        super((...args) => {
          if (name === "ResizeObserver") counters.resizeDeliveries++;
          callback(...args);
        }, options);
      }
      observe(target, options) {
        if (!observerTargets[name].has(this))
          observerTargets[name].set(this, new Set());
        observerTargets[name].get(this).add(target);
        return super.observe(target, options);
      }
      unobserve(target) {
        observerTargets[name].get(this)?.delete(target);
        return super.unobserve(target);
      }
      disconnect() {
        observerTargets[name].delete(this);
        return super.disconnect();
      }
    };
  }
  const rect = Element.prototype.getBoundingClientRect;
  Element.prototype.getBoundingClientRect = function () {
    counters.layouts++;
    return rect.call(this);
  };
  window.requestAnimationFrame = (callback) => {
    let id = nativeRAF((time) => {
      pendingRAF.delete(id);
      counters.raf++;
      callback(time);
    });
    pendingRAF.add(id);
    return id;
  };
  const cancelRAF = window.cancelAnimationFrame.bind(window);
  window.cancelAnimationFrame = (id) => {
    pendingRAF.delete(id);
    cancelRAF(id);
  };
  window.setTimeout = (callback, delay, ...args) => {
    const id = nativeTimeout(() => {
      timers.delete(id);
      counters.timer++;
      typeof callback === "function"
        ? callback(...args)
        : window.eval(callback);
    }, delay);
    timers.add(id);
    return id;
  };
  const clearTimeout = window.clearTimeout.bind(window);
  window.clearTimeout = (id) => {
    timers.delete(id);
    clearTimeout(id);
  };
  const setInterval = window.setInterval.bind(window),
    clearInterval = window.clearInterval.bind(window);
  window.setInterval = (callback, delay, ...args) => {
    const id = setInterval(() => {
      counters.timer++;
      typeof callback === "function"
        ? callback(...args)
        : window.eval(callback);
    }, delay);
    intervals.add(id);
    return id;
  };
  window.clearInterval = (id) => {
    intervals.delete(id);
    clearInterval(id);
  };
  const seenFibers = new WeakMap();
  const hook = {
    supportsFiber: true,
    renderers: new Map(),
    inject(renderer) {
      const id = this.renderers.size + 1;
      this.renderers.set(id, renderer);
      return id;
    },
    onCommitFiberRoot(_id, root) {
      counters.commits++;
      const visit = (fiber) => {
        if (!fiber) return;
        if (typeof fiber.type === "function") {
          const before =
            seenFibers.get(fiber) ||
            (fiber.alternate && seenFibers.get(fiber.alternate));
          if (
            fiber.flags & 1 &&
            (!before ||
              before.props !== fiber.memoizedProps ||
              before.state !== fiber.memoizedState)
          ) {
            const name =
              fiber.type.displayName || fiber.type.name || "anonymous";
            counters.renders[name] = (counters.renders[name] || 0) + 1;
          }
          const current = {
            props: fiber.memoizedProps,
            state: fiber.memoizedState,
          };
          seenFibers.set(fiber, current);
          if (fiber.alternate) seenFibers.set(fiber.alternate, current);
        }
        visit(fiber.child);
        visit(fiber.sibling);
      };
      visit(root.current);
    },
    onCommitFiberUnmount() {},
  };
  if (!window.__REACT_DEVTOOLS_GLOBAL_HOOK__)
    window.__REACT_DEVTOOLS_GLOBAL_HOOK__ = hook;
  let mutationObserver;
  const probe = (window.__kazePerf = {
    counters,
    attach() {
      mutationObserver?.disconnect();
      const app = document.querySelector(".kaze-interface");
      if (!app) return;
      mutationObserver = new MutationObserver((records) => {
        for (const record of records) {
          const element =
            record.target.nodeType === 1
              ? record.target
              : record.target.parentElement;
          const region =
            [
              ".progress-container",
              ".sidebar",
              ".interface-toolbar",
              ".home-view",
              ".discovery-chapters",
              ".playback-stage",
            ].find((selector) => element?.closest(selector)) || "other";
          counters.mutations[region] = (counters.mutations[region] || 0) + 1;
        }
      });
      mutationObserver.observe(app, {
        subtree: true,
        childList: true,
        characterData: true,
        attributes: true,
      });
    },
    pauseMutationCounters() { mutationObserver?.disconnect(); },
    reset() {
      for (const key of Object.keys(counters))
        counters[key] = typeof counters[key] === "number" ? 0 : {};
    },
    snapshot(inventory = false) {
      const result = {
        ...JSON.parse(JSON.stringify(counters)),
        trustedInput,
        route: document.querySelector(".main-content")?.dataset.view,
        visibility: document.visibilityState,
        dom: document.querySelectorAll("*").length,
        images: document.images.length,
        hiddenChapterImages: document.querySelectorAll(
          ".discovery-chapter[hidden] img",
        ).length,
        listeners: Object.fromEntries(
          [...globals]
            .filter(([, value]) => value.size)
            .map(([key, value]) => [key, value.size]),
        ),
        observers: Object.fromEntries(
          Object.entries(observerTargets).map(([name, list]) => [
            name,
            {
              instances: list.size,
              targets: [...list.values()].reduce(
                (n, targets) => n + targets.size,
                0,
              ),
            },
          ]),
        ),
        tasks: {
          raf: pendingRAF.size,
          timers: timers.size,
          intervals: intervals.size,
        },
      };
      if (inventory) {
        result.animations = document.getAnimations().map((animation) => {
          const el = animation.effect?.target,
            box = el ? rect.call(el) : null;
          return {
            name: animation.animationName,
            state: animation.playState,
            target: el?.className?.baseVal || el?.className,
            offscreen:
              box && (box.bottom < 0 || box.top > innerHeight || !box.width),
          };
        });
        result.effects = [
          ...document.querySelectorAll(".kaze-interface *"),
        ].flatMap((el) => {
          const css = getComputedStyle(el);
          return css.filter !== "none" ||
            (css.backdropFilter && css.backdropFilter !== "none") ||
            css.mixBlendMode !== "normal"
            ? [
                {
                  class: el.className?.baseVal || el.className,
                  filter: css.filter,
                  backdrop: css.backdropFilter,
                  blend: css.mixBlendMode,
                },
              ]
            : [];
        });
        result.artworkPixels = [...document.images].reduce(
          (total, image) => total + image.naturalWidth * image.naturalHeight,
          0,
        );
      }
      return result;
    },
    frames(duration = 2000) {
      const gaps = [],
        start = performance.now();
      let last = start;
      const frame = (time) => {
        gaps.push(time - last);
        last = time;
        if (time - start < duration) nativeRAF(frame);
        else {
          gaps.sort((a, b) => a - b);
          probe.frameResult = {
            frames: gaps.length,
            p95: gaps[Math.floor(gaps.length * 0.95)],
            max: gaps.at(-1),
            over32: gaps.filter((n) => n > 32).length,
          };
        }
      };
      probe.frameResult = null;
      nativeRAF(frame);
    },
  });
})();
