import { f as derived, a9 as attributes, aa as bind_props, ab as spread_props } from "./index2.js";
import { c as cn } from "./utils2.js";
import parse from "style-to-object";
import { clsx } from "clsx";
import { h as hasContext, g as getContext, s as setContext } from "./context.js";
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
          result[key] = clsx(a, b);
        } else if (aIsClassValue) {
          result[key] = clsx(a);
        } else if (bIsClassValue) {
          result[key] = clsx(b);
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
    const { window = defaultWindow, document: document2 = window?.document } = options;
    if (window === void 0) return;
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
export {
  Switch as S
};
