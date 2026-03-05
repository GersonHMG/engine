import { a as attributes, c as clsx$1, b as bind_props, ad as element, s as spread_props } from "./index2.js";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import { tv } from "tailwind-variants";
function cn(...inputs) {
  return twMerge(clsx(inputs));
}
function Card($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { class: className, children, $$slots, $$events, ...restProps } = $$props;
    $$renderer2.push(`<div${attributes({
      class: clsx$1(cn("rounded-lg border bg-card text-card-foreground shadow-sm", className)),
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></div>`);
  });
}
function Card_content($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { class: className, children, $$slots, $$events, ...restProps } = $$props;
    $$renderer2.push(`<div${attributes({ class: clsx$1(cn("p-3 pt-0", className)), ...restProps })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></div>`);
  });
}
function Input($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className,
      value = void 0,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    $$renderer2.push(`<input${attributes(
      {
        class: clsx$1(cn("flex h-8 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50", className)),
        value,
        ...restProps
      },
      void 0,
      void 0,
      void 0,
      4
    )}/>`);
    bind_props($$props, { value });
  });
}
function Label($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let { class: className, children, $$slots, $$events, ...restProps } = $$props;
    $$renderer2.push(`<label${attributes({
      class: clsx$1(cn("text-xs font-medium leading-none text-muted-foreground peer-disabled:cursor-not-allowed peer-disabled:opacity-70", className)),
      ...restProps
    })}>`);
    children?.($$renderer2);
    $$renderer2.push(`<!----></label>`);
  });
}
function Button$1($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      href,
      type,
      children,
      disabled = false,
      ref = void 0,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    element(
      $$renderer2,
      href ? "a" : "button",
      () => {
        $$renderer2.push(`${attributes({
          type: href ? void 0 : type,
          href: href && !disabled ? href : void 0,
          disabled: href ? void 0 : disabled,
          "aria-disabled": href ? disabled : void 0,
          role: href && disabled ? "link" : void 0,
          tabindex: href && disabled ? -1 : 0,
          ...restProps
        })}`);
      },
      () => {
        children?.($$renderer2);
        $$renderer2.push(`<!---->`);
      }
    );
    bind_props($$props, { ref });
  });
}
function Button($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let {
      class: className,
      variant = "default",
      size = "default",
      children,
      ref = null,
      $$slots,
      $$events,
      ...restProps
    } = $$props;
    let $$settled = true;
    let $$inner_renderer;
    function $$render_inner($$renderer3) {
      if (Button$1) {
        $$renderer3.push("<!--[-->");
        Button$1($$renderer3, spread_props([
          { class: cn(buttonVariants({ variant, size }), className) },
          restProps,
          {
            get ref() {
              return ref;
            },
            set ref($$value) {
              ref = $$value;
              $$settled = false;
            },
            children: ($$renderer4) => {
              children?.($$renderer4);
              $$renderer4.push(`<!---->`);
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
    bind_props($$props, { ref });
  });
}
const buttonVariants = tv({
  base: "focus-visible:ring-ring inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  variants: {
    variant: {
      default: "bg-primary text-primary-foreground hover:bg-primary/90 shadow",
      destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm",
      outline: "border-input bg-background hover:bg-accent hover:text-accent-foreground border shadow-sm",
      secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-sm",
      ghost: "hover:bg-accent hover:text-accent-foreground",
      link: "text-primary underline-offset-4 hover:underline"
    },
    size: {
      default: "h-9 px-4 py-2",
      sm: "h-8 rounded-md px-3 text-xs",
      lg: "h-10 rounded-md px-8",
      icon: "h-9 w-9"
    }
  },
  defaultVariants: {
    variant: "default",
    size: "default"
  }
});
export {
  Button as B,
  Card as C,
  Input as I,
  Label as L,
  Card_content as a,
  cn as c
};
