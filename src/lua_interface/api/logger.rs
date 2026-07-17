// lua_interface/api/logger.rs — Lua Logger Bridge
// Implements specs/testing_system.md V1.2.1

use crate::logger::Logger;
use mlua::prelude::*;
use std::collections::HashMap;

fn lua_value_to_json(lua: &Lua, val: LuaValue) -> LuaResult<serde_json::Value> {
    match val {
        LuaValue::Nil => Ok(serde_json::Value::Null),
        LuaValue::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        LuaValue::Integer(i) => Ok(serde_json::Value::Number(i.into())),
        LuaValue::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        LuaValue::String(s) => {
            let s_str = s.to_str()?;
            Ok(serde_json::Value::String(s_str.to_string()))
        }
        LuaValue::Table(t) => {
            let len = t.len().unwrap_or(0);
            if len > 0 {
                let mut arr = Vec::new();
                for i in 1..=len {
                    let v: LuaValue = t.get(i)?;
                    arr.push(lua_value_to_json(lua, v)?);
                }
                Ok(serde_json::Value::Array(arr))
            } else {
                let mut obj = serde_json::Map::new();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair?;
                    let key_str = match k {
                        LuaValue::String(s) => s.to_str()?.to_string(),
                        LuaValue::Integer(i) => i.to_string(),
                        LuaValue::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    obj.insert(key_str, lua_value_to_json(lua, v)?);
                }
                Ok(serde_json::Value::Object(obj))
            }
        }
        _ => Ok(serde_json::Value::Null),
    }
}

impl LuaUserData for Logger {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("log_csv", |_, this, data: LuaTable| {
            let mut map = HashMap::new();
            for pair in data.pairs::<LuaValue, LuaValue>() {
                let (k, v) = pair?;
                let key_str = match k {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    LuaValue::Number(n) => n.to_string(),
                    _ => continue,
                };
                let val = match v {
                    LuaValue::Number(n) => crate::logger::LogValue::Float(n),
                    LuaValue::Integer(i) => crate::logger::LogValue::Float(i as f64),
                    LuaValue::Boolean(b) => crate::logger::LogValue::Str(b.to_string()),
                    LuaValue::String(s) => crate::logger::LogValue::Str(s.to_str()?.to_string()),
                    _ => continue,
                };
                map.insert(key_str, val);
            }
            this.log_csv(map).map_err(|e| LuaError::RuntimeError(e.to_string()))
        });

        methods.add_method("log_json", |lua, this, data: LuaValue| {
            let json_val = lua_value_to_json(lua, data)?;
            this.log_json(json_val).map_err(|e| LuaError::RuntimeError(e.to_string()))
        });

        methods.add_method("close", |_, this, ()| {
            if this.is_main {
                let mut reg = crate::logger::get_registry().lock().unwrap();
                reg.stop_session();
            }
            Ok(())
        });
    }
}

pub fn register_logger_functions(lua: &Lua) {
    let logger_table = match lua.create_table() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create Logger table: {}", e);
            return;
        }
    };

    let new_fn = lua.create_function(|_, (log_name, columns_val, is_main): (String, LuaValue, bool)| {
        let columns = match columns_val {
            LuaValue::String(s) => vec![s.to_str()?.to_string()],
            LuaValue::Table(t) => {
                let mut cols = Vec::new();
                for i in 1..=t.len()? {
                    let v: LuaValue = t.get(i)?;
                    let v_str = match v {
                        LuaValue::String(s) => s.to_str()?.to_string(),
                        LuaValue::Integer(i) => i.to_string(),
                        LuaValue::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    cols.push(v_str);
                }
                cols
            }
            _ => return Err(LuaError::RuntimeError("Second argument (columns) must be a string or a table of strings".to_string())),
        };
        let cols_ref: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
        let logger = Logger::new(&log_name, cols_ref, is_main)
            .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
        Ok(logger)
    });

    if let Ok(f) = new_fn {
        let _ = logger_table.set("new", f);
    }

    let _ = lua.globals().set("Logger", logger_table);
}
