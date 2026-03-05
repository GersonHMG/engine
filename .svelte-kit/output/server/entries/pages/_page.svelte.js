import { a as attributes, c as clsx, d as derived, b as bind_props, s as spread_props, e as attr_class, f as stringify, g as store_get, u as unsubscribe_stores } from "../../chunks/index2.js";
import { o as onDestroy } from "../../chunks/index-server.js";
import "@tauri-apps/api/event";
import "@tauri-apps/api/window";
import { w as writable, g as get } from "../../chunks/index.js";
import { clsx as clsx$1 } from "clsx";
import { invoke } from "@tauri-apps/api/tauri";
import { e as escape_html, h as hasContext, g as getContext, s as setContext } from "../../chunks/context.js";
import { c as cn, B as Button, L as Label, I as Input, C as Card, a as Card_content } from "../../chunks/card-content.js";
import { tv } from "tailwind-variants";
import parse from "style-to-object";
import "@tauri-apps/api/dialog";
writable(new Array(50).fill(0));
const manualControlActive = writable(false);
const controlTeam = writable(0);
const controlRobotId = writable(0);
const visualizeVelocities = writable(false);
const pathDrawMode = writable(false);
const pathTraceMode = writable(false);
const pathPoints = writable([]);
const VEL_CHART_SIZE = 600;
writable({
  vx: new Array(VEL_CHART_SIZE).fill(0),
  vy: new Array(VEL_CHART_SIZE).fill(0),
  omega: new Array(VEL_CHART_SIZE).fill(0)
});
const POS_HISTORY_SIZE = 600;
writable({
  x: new Array(POS_HISTORY_SIZE).fill(0),
  y: new Array(POS_HISTORY_SIZE).fill(0),
  theta: new Array(POS_HISTORY_SIZE).fill(0)
});
function screenToField(vp, clientX, clientY, canvasRect) {
  const cx = vp.width / 2 + vp.panX;
  const cy = vp.height / 2 + vp.panY;
  const mouseX = clientX - canvasRect.left;
  const mouseY = clientY - canvasRect.top;
  return {
    x: (mouseX - cx) / vp.scale / 1e3,
    y: -(mouseY - cy) / vp.scale / 1e3
  };
}
function FieldCanvas($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let canvas;
    let mouseCoords = "0, 0";
    let scale = 0.08;
    let panX = 0;
    let panY = 0;
    let isDragging = false;
    let didDrag = false;
    let lastMouseX = 0;
    let lastMouseY = 0;
    function getViewport() {
      return {
        width: canvas.width,
        height: canvas.height,
        panX,
        panY,
        scale
      };
    }
    function resize() {
      return;
    }
    function updateMouseCoords(clientX, clientY) {
      return;
    }
    function onMouseMove(e) {
      updateMouseCoords(e.clientX, e.clientY);
      if (!isDragging) return;
      const dx = e.clientX - lastMouseX;
      const dy = e.clientY - lastMouseY;
      if (Math.abs(dx) > 2 || Math.abs(dy) > 2) {
        didDrag = true;
      }
      panX += dx;
      panY += dy;
      lastMouseX = e.clientX;
      lastMouseY = e.clientY;
    }
    function onMouseUp(e) {
      if (isDragging && !didDrag && e.target === canvas) {
        const vp = getViewport();
        const rect = canvas.getBoundingClientRect();
        const pos = screenToField(vp, e.clientX, e.clientY, rect);
        if (get(pathDrawMode)) {
          pathPoints.update((pts) => [...pts, pos]);
        }
      }
      isDragging = false;
    }
    onDestroy(() => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      window.removeEventListener("resize", resize);
    });
    $$renderer2.push(`<div class="relative flex-1 overflow-hidden bg-[#111]"><canvas class="h-full w-full" style="background-color: #A9A9A9;"></canvas> <div class="pointer-events-none absolute left-4 top-4 rounded bg-black/50 px-2.5 py-1 font-mono text-sm text-white">${escape_html(mouseCoords)}</div> <div class="absolute bottom-4 right-4 flex gap-1.5"><button class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20">-</button> <button class="flex h-8 w-8 items-center justify-center rounded-full bg-white/10 text-lg text-white hover:bg-white/20">+</button></div></div>`);
  });
}
function Badge($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const badgeVariants = tv({
      base: "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
      variants: {
        variant: {
          default: "border-transparent bg-primary text-primary-foreground",
          secondary: "border-transparent bg-secondary text-secondary-foreground",
          destructive: "border-transparent bg-destructive text-destructive-foreground",
          outline: "text-foreground"
        }
      },
      defaultVariants: { variant: "default" }
    });
    let {
      class: className,
      variant = "default",
      children,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    $$renderer2.push(`<div${attributes({
      class: clsx(cn(badgeVariants({ variant }), className)),
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></div>`);
  });
}
function isFunction(value) {
  return typeof value === "function";
}
function isObject(value) {
  return value !== null && typeof value === "object";
}
const CLASS_VALUE_PRIMITIVE_TYPES = ["string", "number", "bigint", "boolean"];
function isClassValue(value) {
  if (value === null || value === void 0)
    return true;
  if (CLASS_VALUE_PRIMITIVE_TYPES.includes(typeof value))
    return true;
  if (Array.isArray(value))
    return value.every((item) => isClassValue(item));
  if (typeof value === "object") {
    if (Object.getPrototypeOf(value) !== Object.prototype)
      return false;
    return true;
  }
  return false;
}
const BoxSymbol = /* @__PURE__ */ Symbol("box");
const isWritableSymbol = /* @__PURE__ */ Symbol("is-writable");
function isBox(value) {
  return isObject(value) && BoxSymbol in value;
}
function isWritableBox(value) {
  return box.isBox(value) && isWritableSymbol in value;
}
function box(initialValue) {
  let current = initialValue;
  return {
    [BoxSymbol]: true,
    [isWritableSymbol]: true,
    get current() {
      return current;
    },
    set current(v) {
      current = v;
    }
  };
}
function boxWith(getter, setter) {
  const derived$1 = derived(getter);
  if (setter) {
    return {
      [BoxSymbol]: true,
      [isWritableSymbol]: true,
      get current() {
        return derived$1();
      },
      set current(v) {
        setter(v);
      }
    };
  }
  return {
    [BoxSymbol]: true,
    get current() {
      return getter();
    }
  };
}
function boxFrom(value) {
  if (box.isBox(value)) return value;
  if (isFunction(value)) return box.with(value);
  return box(value);
}
function boxFlatten(boxes) {
  return Object.entries(boxes).reduce(
    (acc, [key, b]) => {
      if (!box.isBox(b)) {
        return Object.assign(acc, { [key]: b });
      }
      if (box.isWritableBox(b)) {
        Object.defineProperty(acc, key, {
          get() {
            return b.current;
          },
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          set(v) {
            b.current = v;
          }
        });
      } else {
        Object.defineProperty(acc, key, {
          get() {
            return b.current;
          }
        });
      }
      return acc;
    },
    {}
  );
}
function toReadonlyBox(b) {
  if (!box.isWritableBox(b)) return b;
  return {
    [BoxSymbol]: true,
    get current() {
      return b.current;
    }
  };
}
box.from = boxFrom;
box.with = boxWith;
box.flatten = boxFlatten;
box.readonly = toReadonlyBox;
box.isBox = isBox;
box.isWritableBox = isWritableBox;
function composeHandlers(...handlers) {
  return function(e) {
    for (const handler of handlers) {
      if (!handler)
        continue;
      if (e.defaultPrevented)
        return;
      if (typeof handler === "function") {
        handler.call(this, e);
      } else {
        handler.current?.call(this, e);
      }
    }
  };
}
const NUMBER_CHAR_RE = /\d/;
const STR_SPLITTERS = ["-", "_", "/", "."];
function isUppercase(char = "") {
  if (NUMBER_CHAR_RE.test(char))
    return void 0;
  return char !== char.toLowerCase();
}
function splitByCase(str) {
  const parts = [];
  let buff = "";
  let previousUpper;
  let previousSplitter;
  for (const char of str) {
    const isSplitter = STR_SPLITTERS.includes(char);
    if (isSplitter === true) {
      parts.push(buff);
      buff = "";
      previousUpper = void 0;
      continue;
    }
    const isUpper = isUppercase(char);
    if (previousSplitter === false) {
      if (previousUpper === false && isUpper === true) {
        parts.push(buff);
        buff = char;
        previousUpper = isUpper;
        continue;
      }
      if (previousUpper === true && isUpper === false && buff.length > 1) {
        const lastChar = buff.at(-1);
        parts.push(buff.slice(0, Math.max(0, buff.length - 1)));
        buff = lastChar + char;
        previousUpper = isUpper;
        continue;
      }
    }
    buff += char;
    previousUpper = isUpper;
    previousSplitter = isSplitter;
  }
  parts.push(buff);
  return parts;
}
function pascalCase(str) {
  if (!str)
    return "";
  return splitByCase(str).map((p) => upperFirst(p)).join("");
}
function camelCase(str) {
  return lowerFirst(pascalCase(str || ""));
}
function upperFirst(str) {
  return str ? str[0].toUpperCase() + str.slice(1) : "";
}
function lowerFirst(str) {
  return str ? str[0].toLowerCase() + str.slice(1) : "";
}
function cssToStyleObj(css) {
  if (!css)
    return {};
  const styleObj = {};
  function iterator(name, value) {
    if (name.startsWith("-moz-") || name.startsWith("-webkit-") || name.startsWith("-ms-") || name.startsWith("-o-")) {
      styleObj[pascalCase(name)] = value;
      return;
    }
    if (name.startsWith("--")) {
      styleObj[name] = value;
      return;
    }
    styleObj[camelCase(name)] = value;
  }
  parse(css, iterator);
  return styleObj;
}
function executeCallbacks(...callbacks) {
  return (...args) => {
    for (const callback of callbacks) {
      if (typeof callback === "function") {
        callback(...args);
      }
    }
  };
}
function createParser(matcher, replacer) {
  const regex = RegExp(matcher, "g");
  return (str) => {
    if (typeof str !== "string") {
      throw new TypeError(`expected an argument of type string, but got ${typeof str}`);
    }
    if (!str.match(regex))
      return str;
    return str.replace(regex, replacer);
  };
}
const camelToKebab = createParser(/[A-Z]/, (match) => `-${match.toLowerCase()}`);
function styleToCSS(styleObj) {
  if (!styleObj || typeof styleObj !== "object" || Array.isArray(styleObj)) {
    throw new TypeError(`expected an argument of type object, but got ${typeof styleObj}`);
  }
  return Object.keys(styleObj).map((property) => `${camelToKebab(property)}: ${styleObj[property]};`).join("\n");
}
function styleToString(style = {}) {
  return styleToCSS(style).replace("\n", " ");
}
const srOnlyStyles = {
  position: "absolute",
  width: "1px",
  height: "1px",
  padding: "0",
  margin: "-1px",
  overflow: "hidden",
  clip: "rect(0, 0, 0, 0)",
  whiteSpace: "nowrap",
  borderWidth: "0",
  transform: "translateX(-100%)"
};
styleToString(srOnlyStyles);
function isEventHandler(key) {
  return key.length > 2 && key.startsWith("on") && key[2] === key[2]?.toLowerCase();
}
function mergeProps(...args) {
  const result = { ...args[0] };
  for (let i = 1; i < args.length; i++) {
    const props = args[i];
    for (const key in props) {
      const a = result[key];
      const b = props[key];
      const aIsFunction = typeof a === "function";
      const bIsFunction = typeof b === "function";
      if (aIsFunction && typeof bIsFunction && isEventHandler(key)) {
        const aHandler = a;
        const bHandler = b;
        result[key] = composeHandlers(aHandler, bHandler);
      } else if (aIsFunction && bIsFunction) {
        result[key] = executeCallbacks(a, b);
      } else if (key === "class") {
        const aIsClassValue = isClassValue(a);
        const bIsClassValue = isClassValue(b);
        if (aIsClassValue && bIsClassValue) {
          result[key] = clsx$1(a, b);
        } else if (aIsClassValue) {
          result[key] = clsx$1(a);
        } else if (bIsClassValue) {
          result[key] = clsx$1(b);
        }
      } else if (key === "style") {
        const aIsObject = typeof a === "object";
        const bIsObject = typeof b === "object";
        const aIsString = typeof a === "string";
        const bIsString = typeof b === "string";
        if (aIsObject && bIsObject) {
          result[key] = { ...a, ...b };
        } else if (aIsObject && bIsString) {
          const parsedStyle = cssToStyleObj(b);
          result[key] = { ...a, ...parsedStyle };
        } else if (aIsString && bIsObject) {
          const parsedStyle = cssToStyleObj(a);
          result[key] = { ...parsedStyle, ...b };
        } else if (aIsString && bIsString) {
          const parsedStyleA = cssToStyleObj(a);
          const parsedStyleB = cssToStyleObj(b);
          result[key] = { ...parsedStyleA, ...parsedStyleB };
        } else if (aIsObject) {
          result[key] = a;
        } else if (bIsObject) {
          result[key] = b;
        } else if (aIsString) {
          result[key] = a;
        } else if (bIsString) {
          result[key] = b;
        }
      } else {
        result[key] = b !== void 0 ? b : a;
      }
    }
  }
  if (typeof result.style === "object") {
    result.style = styleToString(result.style).replaceAll("\n", " ");
  }
  if (result.hidden !== true) {
    result.hidden = void 0;
    delete result.hidden;
  }
  if (result.disabled !== true) {
    result.disabled = void 0;
    delete result.disabled;
  }
  return result;
}
const defaultWindow = void 0;
function getActiveElement(document2) {
  let activeElement = document2.activeElement;
  while (activeElement?.shadowRoot) {
    const node = activeElement.shadowRoot.activeElement;
    if (node === activeElement)
      break;
    else
      activeElement = node;
  }
  return activeElement;
}
function createSubscriber(_) {
  return () => {
  };
}
class ActiveElement {
  #document;
  #subscribe;
  constructor(options = {}) {
    const { window: window2 = defaultWindow, document: document2 = window2?.document } = options;
    if (window2 === void 0) return;
    this.#document = document2;
    this.#subscribe = createSubscriber();
  }
  get current() {
    this.#subscribe?.();
    if (!this.#document) return null;
    return getActiveElement(this.#document);
  }
}
new ActiveElement();
function runWatcher(sources, flush, effect, options = {}) {
  const { lazy = false } = options;
}
function watch(sources, effect, options) {
  runWatcher(sources, "post", effect, options);
}
function watchPre(sources, effect, options) {
  runWatcher(sources, "pre", effect, options);
}
watch.pre = watchPre;
class Context {
  #name;
  #key;
  /**
   * @param name The name of the context.
   * This is used for generating the context key and error messages.
   */
  constructor(name) {
    this.#name = name;
    this.#key = Symbol(name);
  }
  /**
   * The key used to get and set the context.
   *
   * It is not recommended to use this value directly.
   * Instead, use the methods provided by this class.
   */
  get key() {
    return this.#key;
  }
  /**
   * Checks whether this has been set in the context of a parent component.
   *
   * Must be called during component initialisation.
   */
  exists() {
    return hasContext(this.#key);
  }
  /**
   * Retrieves the context that belongs to the closest parent component.
   *
   * Must be called during component initialisation.
   *
   * @throws An error if the context does not exist.
   */
  get() {
    const context = getContext(this.#key);
    if (context === void 0) {
      throw new Error(`Context "${this.#name}" not found`);
    }
    return context;
  }
  /**
   * Retrieves the context that belongs to the closest parent component,
   * or the given fallback value if the context does not exist.
   *
   * Must be called during component initialisation.
   */
  getOr(fallback) {
    const context = getContext(this.#key);
    if (context === void 0) {
      return fallback;
    }
    return context;
  }
  /**
   * Associates the given value with the current component and returns it.
   *
   * Must be called during component initialisation.
   */
  set(context) {
    return setContext(this.#key, context);
  }
}
function useRefById({ id, ref, deps = () => true, onRefChange, getRootNode }) {
  watch([() => id.current, deps], ([_id]) => {
    const rootNode = getRootNode?.() ?? document;
    const node = rootNode?.getElementById(_id);
    if (node) ref.current = node;
    else ref.current = null;
    onRefChange?.(ref.current);
  });
}
function getDataChecked(condition) {
  return condition ? "checked" : "unchecked";
}
function getDataDisabled(condition) {
  return condition ? "" : void 0;
}
function getAriaRequired(condition) {
  return condition ? "true" : "false";
}
function getAriaChecked(checked, indeterminate) {
  return checked ? "true" : "false";
}
function getAriaHidden(condition) {
  return condition ? "true" : void 0;
}
function getDataRequired(condition) {
  return condition ? "" : void 0;
}
function getDisabled(condition) {
  return condition ? true : void 0;
}
const ENTER = "Enter";
const SPACE = " ";
globalThis.bitsIdCounter ??= { current: 0 };
function useId(prefix = "bits") {
  globalThis.bitsIdCounter.current++;
  return `${prefix}-${globalThis.bitsIdCounter.current}`;
}
function noop() {
}
const SWITCH_ROOT_ATTR = "data-switch-root";
const SWITCH_THUMB_ATTR = "data-switch-thumb";
class SwitchRootState {
  opts;
  constructor(opts) {
    this.opts = opts;
    useRefById(opts);
    this.onkeydown = this.onkeydown.bind(this);
    this.onclick = this.onclick.bind(this);
  }
  #toggle() {
    this.opts.checked.current = !this.opts.checked.current;
  }
  onkeydown(e) {
    if (!(e.key === ENTER || e.key === SPACE) || this.opts.disabled.current) return;
    e.preventDefault();
    this.#toggle();
  }
  onclick(_) {
    if (this.opts.disabled.current) return;
    this.#toggle();
  }
  #sharedProps = derived(() => ({
    "data-disabled": getDataDisabled(this.opts.disabled.current),
    "data-state": getDataChecked(this.opts.checked.current),
    "data-required": getDataRequired(this.opts.required.current)
  }));
  get sharedProps() {
    return this.#sharedProps();
  }
  set sharedProps($$value) {
    return this.#sharedProps($$value);
  }
  #snippetProps = derived(() => ({ checked: this.opts.checked.current }));
  get snippetProps() {
    return this.#snippetProps();
  }
  set snippetProps($$value) {
    return this.#snippetProps($$value);
  }
  #props = derived(() => ({
    ...this.sharedProps,
    id: this.opts.id.current,
    role: "switch",
    disabled: getDisabled(this.opts.disabled.current),
    "aria-checked": getAriaChecked(this.opts.checked.current),
    "aria-required": getAriaRequired(this.opts.required.current),
    [SWITCH_ROOT_ATTR]: "",
    onclick: this.onclick,
    onkeydown: this.onkeydown
  }));
  get props() {
    return this.#props();
  }
  set props($$value) {
    return this.#props($$value);
  }
}
class SwitchInputState {
  root;
  #shouldRender = derived(() => this.root.opts.name.current !== void 0);
  get shouldRender() {
    return this.#shouldRender();
  }
  set shouldRender($$value) {
    return this.#shouldRender($$value);
  }
  constructor(root) {
    this.root = root;
  }
  #props = derived(() => ({
    type: "checkbox",
    name: this.root.opts.name.current,
    value: this.root.opts.value.current,
    checked: this.root.opts.checked.current,
    disabled: this.root.opts.disabled.current,
    required: this.root.opts.required.current,
    "aria-hidden": getAriaHidden(true),
    style: styleToString(srOnlyStyles)
  }));
  get props() {
    return this.#props();
  }
  set props($$value) {
    return this.#props($$value);
  }
}
class SwitchThumbState {
  opts;
  root;
  constructor(opts, root) {
    this.opts = opts;
    this.root = root;
    useRefById(opts);
  }
  #snippetProps = derived(() => ({ checked: this.root.opts.checked.current }));
  get snippetProps() {
    return this.#snippetProps();
  }
  set snippetProps($$value) {
    return this.#snippetProps($$value);
  }
  #props = derived(() => ({
    ...this.root.sharedProps,
    id: this.opts.id.current,
    [SWITCH_THUMB_ATTR]: ""
  }));
  get props() {
    return this.#props();
  }
  set props($$value) {
    return this.#props($$value);
  }
}
const SwitchRootContext = new Context("Switch.Root");
function useSwitchRoot(props) {
  return SwitchRootContext.set(new SwitchRootState(props));
}
function useSwitchInput() {
  return new SwitchInputState(SwitchRootContext.get());
}
function useSwitchThumb(props) {
  return new SwitchThumbState(props, SwitchRootContext.get());
}
function Switch_input($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    const inputState = useSwitchInput();
    if (inputState.shouldRender) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<input${attributes({ ...inputState.props }, void 0, void 0, void 0, 4)}/>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]-->`);
  });
}
function Switch$1($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      child,
      children,
      ref = null,
      id = useId(),
      disabled = false,
      required = false,
      checked = false,
      value = "on",
      name = void 0,
      type = "button",
      onCheckedChange = noop,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const rootState = useSwitchRoot({
      checked: box.with(() => checked, (v) => {
        checked = v;
        onCheckedChange?.(v);
      }),
      disabled: box.with(() => disabled ?? false),
      required: box.with(() => required),
      value: box.with(() => value),
      name: box.with(() => name),
      id: box.with(() => id),
      ref: box.with(() => ref, (v) => ref = v)
    });
    const mergedProps = derived(() => mergeProps(restProps, rootState.props, { type }));
    if (child) {
      $$renderer2.push("<!--[0-->");
      child($$renderer2, { props: mergedProps(), ...rootState.snippetProps });
      $$renderer2.push(`<!---->`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<button${attributes({ ...mergedProps() })}>`);
      children?.($$renderer2, rootState.snippetProps);
      $$renderer2.push(`<!----></button>`);
    }
    $$renderer2.push(`<!--]--> `);
    Switch_input($$renderer2);
    $$renderer2.push(`<!---->`);
    bind_props($$props, { ref, checked });
  });
}
function Switch_thumb($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      child,
      children,
      ref = null,
      id = useId(),
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    const thumbState = useSwitchThumb({
      id: box.with(() => id),
      ref: box.with(() => ref, (v) => ref = v)
    });
    const mergedProps = derived(() => mergeProps(restProps, thumbState.props));
    if (child) {
      $$renderer2.push("<!--[0-->");
      child($$renderer2, { props: mergedProps(), ...thumbState.snippetProps });
      $$renderer2.push(`<!---->`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<span${attributes({ ...mergedProps() })}>`);
      children?.($$renderer2, thumbState.snippetProps);
      $$renderer2.push(`<!----></span>`);
    }
    $$renderer2.push(`<!--]-->`);
    bind_props($$props, { ref });
  });
}
function FieldToolbar($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let visionIp = "224.5.23.2";
    let visionPort = 10020;
    let popupOpen = false;
    let statusText = "Disconnected";
    onDestroy(() => {
    });
    async function reconnect() {
      try {
        await invoke("update_vision_connection", { ip: visionIp, port: visionPort });
        popupOpen = false;
      } catch (e) {
        console.error("Failed to update vision:", e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      $$renderer3.push(`<div class="flex h-9 shrink-0 items-center gap-3 border-b border-border bg-card px-3"><div class="relative">`);
      Button($$renderer3, {
        variant: "ghost",
        size: "sm",
        class: "h-7 gap-1.5 px-2 text-xs",
        onclick: () => popupOpen = !popupOpen,
        children: ($$renderer4) => {
          $$renderer4.push(`<svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg> Vision`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> `);
      if (popupOpen) {
        $$renderer3.push("<!--[0-->");
        $$renderer3.push(`<div class="absolute left-0 top-full z-50 mt-1 w-64 rounded-md border border-border bg-card p-3 shadow-lg"><h4 class="mb-2 text-xs font-semibold text-muted-foreground">Vision Connection</h4> <div class="space-y-2"><div class="flex items-center justify-between gap-2">`);
        Label($$renderer3, {
          class: "text-xs",
          children: ($$renderer4) => {
            $$renderer4.push(`<!---->IP`);
          },
          $$slots: { default: true }
        });
        $$renderer3.push(`<!----> `);
        Input($$renderer3, {
          type: "text",
          class: "h-7 w-36 text-xs",
          get value() {
            return visionIp;
          },
          set value($$value) {
            visionIp = $$value;
            $$settled = false;
          }
        });
        $$renderer3.push(`<!----></div> <div class="flex items-center justify-between gap-2">`);
        Label($$renderer3, {
          class: "text-xs",
          children: ($$renderer4) => {
            $$renderer4.push(`<!---->Port`);
          },
          $$slots: { default: true }
        });
        $$renderer3.push(`<!----> `);
        Input($$renderer3, {
          type: "number",
          class: "h-7 w-24 text-xs",
          get value() {
            return visionPort;
          },
          set value($$value) {
            visionPort = $$value;
            $$settled = false;
          }
        });
        $$renderer3.push(`<!----></div> `);
        Button($$renderer3, {
          size: "sm",
          class: "h-7 w-full text-xs",
          onclick: reconnect,
          children: ($$renderer4) => {
            $$renderer4.push(`<!---->Reconnect`);
          },
          $$slots: { default: true }
        });
        $$renderer3.push(`<!----></div></div>`);
      } else {
        $$renderer3.push("<!--[-1-->");
      }
      $$renderer3.push(`<!--]--></div> <div class="h-4 w-px bg-border"></div> `);
      Badge($$renderer3, {
        variant: "destructive",
        class: "h-5 text-[10px]",
        children: ($$renderer4) => {
          $$renderer4.push(`<!---->${escape_html(statusText)}`);
        },
        $$slots: { default: true }
      });
      $$renderer3.push(`<!----> <canvas width="120" height="24" class="rounded border border-border bg-[#222]"></canvas></div>`);
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
function Card_header($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { class: className, children, $$slots, $$events, ...restProps } = $$props;
    $$renderer2.push(`<div${attributes({
      class: clsx(cn("flex flex-col space-y-1.5 p-3", className)),
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></div>`);
  });
}
function Card_title($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { class: className, children, $$slots, $$events, ...restProps } = $$props;
    $$renderer2.push(`<h4${attributes({
      class: clsx(cn("text-xs font-medium uppercase tracking-wide text-muted-foreground", className)),
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></h4>`);
  });
}
function Switch($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className,
      checked = false,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      if (Switch$1) {
        $$renderer3.push("<!--[-->");
        Switch$1($$renderer3, spread_props([
          {
            class: cn("peer inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50", checked ? "bg-primary" : "bg-input", className)
          },
          restProps,
          {
            get checked() {
              return checked;
            },
            set checked($$value) {
              checked = $$value;
              $$settled = false;
            },
            children: ($$renderer4) => {
              if (Switch_thumb) {
                $$renderer4.push("<!--[-->");
                Switch_thumb($$renderer4, {
                  class: cn("pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg ring-0 transition-transform", checked ? "translate-x-4" : "translate-x-0")
                });
                $$renderer4.push("<!--]-->");
              } else {
                $$renderer4.push("<!--[!-->");
                $$renderer4.push("<!--]-->");
              }
            },
            $$slots: { default: true }
          }
        ]));
        $$renderer3.push("<!--]-->");
      } else {
        $$renderer3.push("<!--[!-->");
        $$renderer3.push("<!--]-->");
      }
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
    bind_props($$props, { checked });
  });
}
function RadioPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let portName = "/dev/ttyUSB0";
    let baudRate = 115200;
    let useRadio = false;
    async function updateRadio() {
      try {
        await invoke("update_radio_config", { useRadio, portName, baudRate });
        alert("Radio configuration updated!");
      } catch (e) {
        console.error("Failed to update radio:", e);
        alert("Failed to update radio: " + e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_header($$renderer4, {
            children: ($$renderer5) => {
              Card_title($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Radio Configuration`);
                },
                $$slots: { default: true }
              });
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!----> `);
          Card_content($$renderer4, {
            class: "space-y-3",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Port Name`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "text",
                class: "w-32 text-right",
                get value() {
                  return portName;
                },
                set value($$value) {
                  portName = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Baud Rate`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                class: "w-24 text-right",
                get value() {
                  return baudRate;
                },
                set value($$value) {
                  baudRate = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Use Radio`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return useRadio;
                },
                set checked($$value) {
                  useRadio = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> `);
              Button($$renderer5, {
                class: "w-full",
                onclick: updateRadio,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Update Radio`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!---->`);
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!---->`);
        },
        $$slots: { default: true }
      });
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
function KalmanFilterPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let enabled = true;
    let processNoiseP = 1e-7;
    let processNoiseV = 1e-4;
    let measurementNoise = 1e-6;
    async function updateKF() {
      try {
        await invoke("update_tracker_config", { enabled, processNoiseP, processNoiseV, measurementNoise });
      } catch (e) {
        console.error("Failed to update KF:", e);
      }
    }
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_header($$renderer4, {
            children: ($$renderer5) => {
              Card_title($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Kalman Filter`);
                },
                $$slots: { default: true }
              });
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!----> `);
          Card_content($$renderer4, {
            class: "space-y-3",
            children: ($$renderer5) => {
              $$renderer5.push(`<div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Enabled`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Switch($$renderer5, {
                get checked() {
                  return enabled;
                },
                set checked($$value) {
                  enabled = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Proc. Noise (P)`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.0000001",
                class: "w-28 text-right",
                get value() {
                  return processNoiseP;
                },
                set value($$value) {
                  processNoiseP = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Proc. Noise (V)`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.0001",
                class: "w-28 text-right",
                get value() {
                  return processNoiseV;
                },
                set value($$value) {
                  processNoiseV = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> <div class="flex items-center justify-between">`);
              Label($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Meas. Noise`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Input($$renderer5, {
                type: "number",
                step: "0.000001",
                class: "w-28 text-right",
                get value() {
                  return measurementNoise;
                },
                set value($$value) {
                  measurementNoise = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----></div> `);
              Button($$renderer5, {
                class: "w-full",
                onclick: updateKF,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Update KF`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!---->`);
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!---->`);
        },
        $$slots: { default: true }
      });
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
function RecordingPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let filename = "record.csv";
    let status = "idle";
    let startDisabled = false;
    let stopDisabled = true;
    async function startRecording() {
      try {
        await invoke("start_recording", { filename });
        status = "recording";
        startDisabled = true;
        stopDisabled = false;
      } catch (e) {
        console.error(e);
        alert(String(e));
      }
    }
    async function stopRecording() {
      try {
        await invoke("stop_recording");
        status = "saved";
        startDisabled = false;
        stopDisabled = true;
      } catch (e) {
        console.error(e);
      }
    }
    const statusColor = derived(() => status === "recording" ? "text-green-500" : status === "saved" ? "text-muted-foreground" : "text-muted-foreground");
    const statusText = derived(() => status === "recording" ? "Recording..." : status === "saved" ? "Saved" : "Idle");
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      Card($$renderer3, {
        children: ($$renderer4) => {
          Card_header($$renderer4, {
            children: ($$renderer5) => {
              Card_title($$renderer5, {
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Recording`);
                },
                $$slots: { default: true }
              });
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!----> `);
          Card_content($$renderer4, {
            class: "space-y-3",
            children: ($$renderer5) => {
              Input($$renderer5, {
                type: "text",
                class: "w-full",
                get value() {
                  return filename;
                },
                set value($$value) {
                  filename = $$value;
                  $$settled = false;
                }
              });
              $$renderer5.push(`<!----> <div class="flex gap-2">`);
              Button($$renderer5, {
                class: "flex-1",
                disabled: startDisabled,
                onclick: startRecording,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Start`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----> `);
              Button($$renderer5, {
                variant: "destructive",
                class: "flex-1",
                disabled: stopDisabled,
                onclick: stopRecording,
                children: ($$renderer6) => {
                  $$renderer6.push(`<!---->Stop`);
                },
                $$slots: { default: true }
              });
              $$renderer5.push(`<!----></div> <p${attr_class(`text-xs ${stringify(statusColor())}`)}>${escape_html(statusText())}</p>`);
            },
            $$slots: { default: true }
          });
          $$renderer4.push(`<!---->`);
        },
        $$slots: { default: true }
      });
    }
    do {
      $$settled = true;
      $$inner_renderer = $$renderer2.copy();
      $$render_inner($$inner_renderer);
    } while (!$$settled);
    $$renderer2.subsume($$inner_renderer);
  });
}
function BottomPanel($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let capturing = false;
    function toggleTrace() {
      pathTraceMode.update((v) => !v);
    }
    function toggleVectors() {
      visualizeVelocities.update((v) => !v);
    }
    function toggleCapture() {
      capturing = !capturing;
    }
    onDestroy(() => {
    });
    $$renderer2.push(`<div class="flex h-44 shrink-0 items-stretch gap-2 border-t border-border bg-card px-3 py-2"><div class="flex w-24 shrink-0 flex-col justify-center gap-1.5"><span class="text-center text-[10px] font-semibold text-muted-foreground">Robot ${escape_html(store_get($$store_subs ??= {}, "$controlRobotId", controlRobotId))} · ${escape_html(store_get($$store_subs ??= {}, "$controlTeam", controlTeam) === 0 ? "Blue" : "Yellow")}</span> `);
    Button($$renderer2, {
      variant: capturing ? "default" : "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleCapture,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html(capturing ? "Capture ON" : "Capture OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleTrace,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html("Trace OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----> `);
    Button($$renderer2, {
      variant: "secondary",
      size: "sm",
      class: "h-7 text-[10px]",
      onclick: toggleVectors,
      children: ($$renderer3) => {
        $$renderer3.push(`<!---->${escape_html("Vectors OFF")}`);
      },
      $$slots: { default: true }
    });
    $$renderer2.push(`<!----></div> <div class="w-px shrink-0 bg-border"></div> <div class="flex min-w-0 flex-1 gap-1"><div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-red-400">Vx</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-orange-400">X</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div> <div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-green-400">Vy</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-cyan-400">Y</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div> <div class="flex flex-1 flex-col gap-0.5"><div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-blue-400">ω</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div> <div class="flex flex-1 flex-col"><span class="text-[9px] font-bold text-purple-400">θ</span> <canvas width="200" height="36" class="h-full w-full rounded border border-border"></canvas></div></div></div></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    onDestroy(() => {
    });
    $$renderer2.push(`<div class="flex h-screen flex-col overflow-hidden rounded-lg border border-border bg-background text-foreground"><div class="flex h-8 shrink-0 items-center justify-between bg-card" data-tauri-drag-region=""><span class="pointer-events-none select-none pl-3 text-xs font-medium text-muted-foreground" data-tauri-drag-region="">Sysmic Engine</span> <div class="flex h-full"><button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted" aria-label="Minimize"><svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"></rect></svg></button> <button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted" aria-label="Maximize"><svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9"></rect></svg></button> <button class="inline-flex h-full w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-red-600 hover:text-white" aria-label="Close"><svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2"><line x1="0" y1="0" x2="10" y2="10"></line><line x1="10" y1="0" x2="0" y2="10"></line></svg></button></div></div> <nav class="flex h-10 shrink-0 items-center gap-1 border-b border-border bg-card px-4"><button${attr_class(`flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors ${stringify(
      "border-primary text-foreground"
    )}`)}>Connection</button> <button${attr_class(`flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors ${stringify("border-transparent text-muted-foreground hover:text-foreground")}`)}>Control `);
    if (store_get($$store_subs ??= {}, "$manualControlActive", manualControlActive)) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<span class="h-2.5 w-2.5 rounded-full bg-green-500 shadow-[0_0_5px_theme(colors.green.500)]"></span>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></button> <button${attr_class(`flex h-full items-center gap-2 border-b-2 px-4 text-sm font-semibold transition-colors ${stringify("border-transparent text-muted-foreground hover:text-foreground")}`)}>Script</button></nav> <div class="flex flex-1 overflow-hidden"><aside class="flex w-72 shrink-0 flex-col gap-3 overflow-y-auto border-r border-border bg-card p-3"><h3 class="border-b border-border pb-2 text-sm font-semibold">Configuration</h3> `);
    {
      $$renderer2.push("<!--[0-->");
      RadioPanel($$renderer2);
      $$renderer2.push(`<!----> `);
      KalmanFilterPanel($$renderer2);
      $$renderer2.push(`<!----> `);
      RecordingPanel($$renderer2);
      $$renderer2.push(`<!---->`);
    }
    $$renderer2.push(`<!--]--></aside> <div class="flex flex-1 flex-col overflow-hidden">`);
    FieldToolbar($$renderer2);
    $$renderer2.push(`<!----> `);
    FieldCanvas($$renderer2);
    $$renderer2.push(`<!----></div></div> `);
    BottomPanel($$renderer2);
    $$renderer2.push(`<!----></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
