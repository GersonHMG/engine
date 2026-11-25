// lua_interface/ui.rs — Controls a script declares for the GUI (`ui` table)
//
// A script declares parameters (sliders, numbers, toggles, choices) and
// buttons; the GUI shows them in the Script panel and sends back what the
// user changes or presses, and field clicks while the script listens to them.
//
//     local speed = ui.slider("speed", 1.5, 0.1, 4.0)   -- name, default, min, max[, step]
//     local id    = ui.number("robot", 0)
//     local mode  = ui.choice("mode", "goto", {"goto", "spin"})
//     local debug = ui.toggle("debug", false)
//     ui.button("reset", function() ... end)
//     ui.on_click(function(x, y, button) ... end)       -- field click, m; "left" | "right"
//
//     function process()
//         local v = ui.get("speed")    -- the value the user set, now
//     end
//
// A declaration returns the current value and can run at the top level (the
// controls show while the loaded script waits to be started) or inside
// `process()` (it then reads the value every tick). Values outlive reloads:
// a control declared again with the same name and kind keeps what the user
// set, so a reload starts the script with them.

use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Registry key of the table `name -> function` of the script's buttons.
const BUTTONS_KEY: &str = "ui.buttons";
/// Registry key of the function `ui.on_click` registered.
const ON_CLICK_KEY: &str = "ui.on_click";

#[derive(Debug, Clone, PartialEq)]
pub enum UiValue {
    Number(f64),
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiKind {
    Slider { min: f64, max: f64, step: f64 },
    Number,
    Toggle,
    Choice { options: Vec<String> },
    Button,
}

impl UiKind {
    fn same_variant(&self, other: &UiKind) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// `value` as this kind takes it (a slider clamps it), or None if it is
    /// of another type or not one of the options.
    fn accept(&self, value: &UiValue) -> Option<UiValue> {
        match (self, value) {
            (UiKind::Slider { min, max, .. }, UiValue::Number(v)) if v.is_finite() => Some(UiValue::Number(v.clamp(*min, *max))),
            (UiKind::Number, UiValue::Number(v)) if v.is_finite() => Some(UiValue::Number(*v)),
            (UiKind::Toggle, UiValue::Bool(b)) => Some(UiValue::Bool(*b)),
            (UiKind::Choice { options }, UiValue::Text(s)) if options.contains(s) => Some(UiValue::Text(s.clone())),
            (UiKind::Button, _) => Some(UiValue::Bool(false)),
            _ => None,
        }
    }
}

/// One control, in the order the script declared it.
#[derive(Debug, Clone, PartialEq)]
pub struct UiWidget {
    pub name: String,
    pub kind: UiKind,
    pub value: UiValue,
}

/// What the Script panel shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptUiView {
    pub widgets: Vec<UiWidget>,
    /// The script listens to field clicks (`ui.on_click`).
    pub on_click: bool,
}

/// Mouse button of a field click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickButton {
    Left,
    Right,
}

impl ClickButton {
    fn word(self) -> &'static str {
        match self {
            ClickButton::Left => "left",
            ClickButton::Right => "right",
        }
    }
}

/// What the user did in the GUI.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    Set { name: String, value: UiValue },
    Press { name: String },
    /// A click on the field, m.
    Click { x: f64, y: f64, button: ClickButton },
}

/// The controls of the loaded script, shared by the Lua functions and the
/// engine.
#[derive(Debug, Default)]
pub struct ScriptUi {
    widgets: Vec<UiWidget>,
    /// Controls of the previous load, by name: their values come back when
    /// the script declares them again.
    saved: HashMap<String, UiWidget>,
    on_click: bool,
    /// Changed since the last `take_view`.
    dirty: bool,
}

impl ScriptUi {
    /// A script (re)loads: its controls are put aside until it declares them.
    pub fn begin_load(&mut self) {
        for widget in self.widgets.drain(..) {
            self.saved.insert(widget.name.clone(), widget);
        }
        self.on_click = false;
        self.dirty = true;
    }

    /// Declares a control and returns its value: the one it has, the one
    /// it had before the reload if its kind is the same, or `default`.
    pub fn declare(&mut self, name: &str, kind: UiKind, default: UiValue) -> Result<UiValue, String> {
        let default = kind
            .accept(&default)
            .ok_or_else(|| format!("ui: bad default for \"{name}\""))?;

        if let Some(widget) = self.widgets.iter_mut().find(|w| w.name == name) {
            if widget.kind != kind {
                // Declared again with another range or options: keep the
                // value if it still fits.
                let value = if widget.kind.same_variant(&kind) { kind.accept(&widget.value) } else { None };
                widget.value = value.unwrap_or(default);
                widget.kind = kind;
                self.dirty = true;
            }
            return Ok(widget.value.clone());
        }

        let value = self
            .saved
            .remove(name)
            .filter(|old| old.kind.same_variant(&kind))
            .and_then(|old| kind.accept(&old.value))
            .unwrap_or(default);
        self.widgets.push(UiWidget { name: name.to_string(), kind, value: value.clone() });
        self.dirty = true;
        Ok(value)
    }

    pub fn get(&self, name: &str) -> Option<UiValue> {
        self.widgets.iter().find(|w| w.name == name).map(|w| w.value.clone())
    }

    /// Sets a control's value (a slider clamps it). Errors if there is no
    /// such control or the value is not of its type.
    pub fn set(&mut self, name: &str, value: UiValue) -> Result<(), String> {
        let widget = self
            .widgets
            .iter_mut()
            .find(|w| w.name == name)
            .ok_or_else(|| format!("ui: no control named \"{name}\""))?;
        let value = widget
            .kind
            .accept(&value)
            .ok_or_else(|| format!("ui: {value:?} does not fit \"{name}\""))?;
        if widget.value != value {
            widget.value = value;
            self.dirty = true;
        }
        Ok(())
    }

    fn set_on_click(&mut self, listening: bool) {
        if self.on_click != listening {
            self.on_click = listening;
            self.dirty = true;
        }
    }

    pub fn view(&self) -> ScriptUiView {
        ScriptUiView { widgets: self.widgets.clone(), on_click: self.on_click }
    }

    /// The view, if it changed since the last call.
    pub fn take_view(&mut self) -> Option<ScriptUiView> {
        std::mem::take(&mut self.dirty).then(|| self.view())
    }
}

impl IntoLua for UiValue {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        Ok(match self {
            UiValue::Number(v) => LuaValue::Number(v),
            UiValue::Bool(b) => LuaValue::Boolean(b),
            UiValue::Text(s) => LuaValue::String(lua.create_string(&s)?),
        })
    }
}

fn from_lua(value: LuaValue) -> LuaResult<UiValue> {
    match value {
        LuaValue::Integer(v) => Ok(UiValue::Number(v as f64)),
        LuaValue::Number(v) => Ok(UiValue::Number(v)),
        LuaValue::Boolean(b) => Ok(UiValue::Bool(b)),
        LuaValue::String(s) => Ok(UiValue::Text(s.to_str()?.to_string())),
        other => Err(LuaError::external(format!("ui: a value must be a number, boolean or string, not {}", other.type_name()))),
    }
}

fn lock(ui: &Mutex<ScriptUi>) -> std::sync::MutexGuard<'_, ScriptUi> {
    ui.lock().unwrap_or_else(|e| e.into_inner())
}

/// Registers the `ui` table.
pub(super) fn register_ui_functions(lua: &Lua, ui: Arc<Mutex<ScriptUi>>) -> LuaResult<()> {
    let table = lua.create_table()?;
    lua.set_named_registry_value(BUTTONS_KEY, lua.create_table()?)?;

    let declare = |ui: &Arc<Mutex<ScriptUi>>, name: &str, kind: UiKind, default: UiValue| {
        lock(ui).declare(name, kind, default).map_err(LuaError::external)
    };

    // ui.slider(name, default, min, max[, step])
    {
        let ui = Arc::clone(&ui);
        table.set(
            "slider",
            lua.create_function(move |_, (name, default, min, max, step): (String, f64, f64, f64, Option<f64>)| {
                if !(min.is_finite() && max.is_finite() && min < max) {
                    return Err(LuaError::external(format!("ui.slider \"{name}\": min must be below max")));
                }
                let step = step.filter(|s| s.is_finite() && *s > 0.0).unwrap_or((max - min) / 100.0);
                declare(&ui, &name, UiKind::Slider { min, max, step }, UiValue::Number(default))
            })?,
        )?;
    }

    // ui.number(name, default)
    {
        let ui = Arc::clone(&ui);
        table.set(
            "number",
            lua.create_function(move |_, (name, default): (String, f64)| {
                declare(&ui, &name, UiKind::Number, UiValue::Number(default))
            })?,
        )?;
    }

    // ui.toggle(name, default)
    {
        let ui = Arc::clone(&ui);
        table.set(
            "toggle",
            lua.create_function(move |_, (name, default): (String, Option<bool>)| {
                declare(&ui, &name, UiKind::Toggle, UiValue::Bool(default.unwrap_or(false)))
            })?,
        )?;
    }

    // ui.choice(name, default, {options})
    {
        let ui = Arc::clone(&ui);
        table.set(
            "choice",
            lua.create_function(move |_, (name, default, options): (String, String, Vec<String>)| {
                if !options.contains(&default) {
                    return Err(LuaError::external(format!("ui.choice \"{name}\": the default must be one of the options")));
                }
                declare(&ui, &name, UiKind::Choice { options }, UiValue::Text(default))
            })?,
        )?;
    }

    // ui.button(name, fn)
    {
        let ui = Arc::clone(&ui);
        table.set(
            "button",
            lua.create_function(move |lua, (name, callback): (String, LuaFunction)| {
                declare(&ui, &name, UiKind::Button, UiValue::Bool(false))?;
                lua.named_registry_value::<LuaTable>(BUTTONS_KEY)?.set(name, callback)
            })?,
        )?;
    }

    // ui.on_click(fn | nil)
    {
        let ui = Arc::clone(&ui);
        table.set(
            "on_click",
            lua.create_function(move |lua, callback: Option<LuaFunction>| {
                lock(&ui).set_on_click(callback.is_some());
                lua.set_named_registry_value(ON_CLICK_KEY, callback)
            })?,
        )?;
    }

    // ui.get(name) -> value | nil
    {
        let ui = Arc::clone(&ui);
        table.set("get", lua.create_function(move |_, name: String| Ok(lock(&ui).get(&name)))?)?;
    }

    // ui.set(name, value)
    {
        let ui = Arc::clone(&ui);
        table.set(
            "set",
            lua.create_function(move |_, (name, value): (String, LuaValue)| {
                lock(&ui).set(&name, from_lua(value)?).map_err(LuaError::external)
            })?,
        )?;
    }

    lua.globals().set("ui", table)
}

/// Runs the callback a press or click is for, if the script has one.
pub(super) fn dispatch(lua: &Lua, event: &UiEvent) -> LuaResult<()> {
    match event {
        UiEvent::Set { .. } => Ok(()),
        UiEvent::Press { name } => {
            let buttons: LuaTable = lua.named_registry_value(BUTTONS_KEY)?;
            match buttons.get::<Option<LuaFunction>>(name.as_str())? {
                Some(callback) => callback.call(()),
                None => Ok(()),
            }
        }
        UiEvent::Click { x, y, button } => match lua.named_registry_value::<Option<LuaFunction>>(ON_CLICK_KEY)? {
            Some(callback) => callback.call((*x, *y, button.word())),
            None => Ok(()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Lua, Arc<Mutex<ScriptUi>>) {
        let lua = Lua::new();
        let ui = Arc::new(Mutex::new(ScriptUi::default()));
        register_ui_functions(&lua, Arc::clone(&ui)).unwrap();
        (lua, ui)
    }

    #[test]
    fn declarations_return_defaults_in_order() {
        let (lua, ui) = setup();
        let (speed, on, mode): (f64, bool, String) = lua
            .load(r#"return ui.slider("speed", 1.5, 0, 4), ui.toggle("on", true), ui.choice("mode", "b", {"a", "b"})"#)
            .eval()
            .unwrap();
        assert_eq!((speed, on, mode.as_str()), (1.5, true, "b"));
        let names: Vec<_> = lock(&ui).view().widgets.into_iter().map(|w| w.name).collect();
        assert_eq!(names, ["speed", "on", "mode"]);
    }

    #[test]
    fn values_set_by_the_gui_are_read_and_clamped() {
        let (lua, ui) = setup();
        lua.load(r#"ui.slider("speed", 1, 0, 2)"#).exec().unwrap();
        lock(&ui).set("speed", UiValue::Number(9.0)).unwrap();
        assert_eq!(lua.load(r#"return ui.get("speed")"#).eval::<f64>().unwrap(), 2.0);
        // Declaring it again returns the value set, not the default.
        assert_eq!(lua.load(r#"return ui.slider("speed", 1, 0, 2)"#).eval::<f64>().unwrap(), 2.0);
        assert!(lock(&ui).set("speed", UiValue::Bool(true)).is_err());
        assert!(lock(&ui).set("nope", UiValue::Number(1.0)).is_err());
    }

    #[test]
    fn values_survive_a_reload_when_the_kind_matches() {
        let mut ui = ScriptUi::default();
        ui.declare("speed", UiKind::Number, UiValue::Number(1.0)).unwrap();
        ui.declare("mode", UiKind::Choice { options: vec!["a".into(), "b".into()] }, UiValue::Text("a".into())).unwrap();
        ui.set("speed", UiValue::Number(3.0)).unwrap();
        ui.set("mode", UiValue::Text("b".into())).unwrap();

        ui.begin_load();
        assert!(ui.view().widgets.is_empty());
        assert_eq!(ui.declare("speed", UiKind::Number, UiValue::Number(1.0)).unwrap(), UiValue::Number(3.0));
        // "b" is no longer an option: back to the default.
        let mode = ui.declare("mode", UiKind::Choice { options: vec!["a".into(), "c".into()] }, UiValue::Text("a".into()));
        assert_eq!(mode.unwrap(), UiValue::Text("a".into()));
    }

    #[test]
    fn take_view_reports_changes_once() {
        let mut ui = ScriptUi::default();
        ui.declare("speed", UiKind::Number, UiValue::Number(1.0)).unwrap();
        assert!(ui.take_view().is_some());
        assert!(ui.take_view().is_none());
        ui.declare("speed", UiKind::Number, UiValue::Number(1.0)).unwrap();
        ui.set("speed", UiValue::Number(1.0)).unwrap();
        assert!(ui.take_view().is_none());
        ui.set("speed", UiValue::Number(2.0)).unwrap();
        assert!(ui.take_view().is_some());
    }

    #[test]
    fn buttons_and_clicks_run_their_callbacks() {
        let (lua, ui) = setup();
        lua.load(
            r#"
            presses, click = 0, nil
            ui.button("go", function() presses = presses + 1 end)
            ui.on_click(function(x, y, b) click = {x, y, b} end)
            "#,
        )
        .exec()
        .unwrap();
        assert!(lock(&ui).view().on_click);

        dispatch(&lua, &UiEvent::Press { name: "go".into() }).unwrap();
        dispatch(&lua, &UiEvent::Press { name: "missing".into() }).unwrap();
        dispatch(&lua, &UiEvent::Click { x: 1.0, y: -0.5, button: ClickButton::Right }).unwrap();

        assert_eq!(lua.load("return presses").eval::<i64>().unwrap(), 1);
        let (x, y, b): (f64, f64, String) = lua.load("return click[1], click[2], click[3]").eval().unwrap();
        assert_eq!((x, y, b.as_str()), (1.0, -0.5, "right"));

        lua.load("ui.on_click(nil)").exec().unwrap();
        assert!(!lock(&ui).view().on_click);
    }

    #[test]
    fn bad_declarations_are_errors() {
        let (lua, _) = setup();
        assert!(lua.load(r#"ui.slider("s", 1, 2, 1)"#).exec().is_err());
        assert!(lua.load(r#"ui.choice("c", "x", {"a"})"#).exec().is_err());
        assert!(lua.load(r#"ui.number("n", "fast")"#).exec().is_err());
    }
}
