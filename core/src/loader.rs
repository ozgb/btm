//! The module loader of docs/module-abi.md, sections 4, 5, 10, 11 and 13, as the core gate's `loader` and `fuel` groups judge it.
use crate::Call;
use serde_json::Value;
use std::collections::HashMap;
use wasmi::{ValType, Engine, Extern, Func, Linker, Memory, MemoryType, Module, Ref, RefType, Store, Table, TableType, TrapCode, Val};

/// An error class of the gate and the modules it names.
#[derive(Debug, Clone)]
pub struct Err {
    pub class: &'static str,
    pub modules: Vec<String>,
}

impl Err {
    pub fn new(class: &'static str) -> Err {
        Err { class, modules: Vec::new() }
    }
    pub fn named(class: &'static str, name: &str) -> Err {
        Err { class, modules: vec![name.to_string()] }
    }
    pub fn json(&self) -> String {
        let names: Vec<String> = self.modules.iter().map(|m| crate::commands::json_str(m)).collect();
        format!("{{\"error\":\"{}\",\"modules\":[{}]}}", self.class, names.join(","))
    }
}

pub struct Entry {
    pub name: String,
    pub data_base: u64,
    pub table_base: u64,
    pub exports: Vec<String>,
    pub data: Vec<String>,
    pub fuel_budget: Option<i64>,
    pub raw: Value,
}

pub struct Manifest {
    pub raw: Value,
    pub memory_pages: u64,
    pub table_size: u64,
    pub window: u64,
    pub table_slots: u64,
    pub block_base: Option<u64>,
    pub block_size: u64,
    pub block_max: u64,
    pub scratch: Option<(u64, u64)>,
    pub modules: Vec<Entry>,
}

fn bad_number(v: &Value) -> bool {
    match v {
        Value::Number(n) => n.is_f64() || n.as_u64().map(|x| x > 2147483647).unwrap_or(false),
        Value::Array(a) => a.iter().any(bad_number),
        Value::Object(o) => o.values().any(bad_number),
        _ => false,
    }
}

fn nonneg(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

fn names(v: &Value) -> Option<Vec<String>> {
    v.as_array()?.iter().map(|x| x.as_str().map(|s| s.to_string())).collect()
}

/// Reads and checks a manifest, reporting the first fault in the order of docs/core-gate.md, "Which error comes first".
pub fn parse_manifest(text: &[u8]) -> Result<Manifest, Err> {
    let m = || Err::new("manifest");
    let raw: Value = serde_json::from_slice(text).map_err(|_| m())?;
    if bad_number(&raw) {
        return Result::Err(m());
    }
    let obj = raw.as_object().ok_or_else(m)?;
    let mut req = [0u64; 4];
    for (i, k) in ["memory_pages", "table_size", "window", "table_slots"].iter().enumerate() {
        req[i] = obj.get(*k).and_then(nonneg).ok_or_else(m)?;
    }
    let [memory_pages, table_size, window, table_slots] = req;
    if memory_pages == 0 || memory_pages > 65536 || window == 0 {
        return Result::Err(m());
    }
    let mem_bytes = memory_pages * 65536;
    // the block
    let mut block_base = None;
    let (mut block_size, mut block_max) = (0u64, 0u64);
    if let Some(bb) = obj.get("block_base") {
        let bb = nonneg(bb).ok_or_else(m)?;
        block_size = obj.get("block_size").and_then(nonneg).ok_or_else(m)?;
        block_max = block_size;
        if let Some(bm) = obj.get("block_max") {
            block_max = nonneg(bm).ok_or_else(m)?;
        }
        if block_max < block_size {
            return Result::Err(Err::new("layout"));
        }
        if bb + block_size > mem_bytes {
            return Result::Err(Err::new("layout"));
        }
        block_base = Some(bb);
    }
    // the scratch area
    let mut scratch = None;
    if let Some(s) = obj.get("scratch") {
        let a = s.as_array().ok_or_else(m)?;
        if a.len() != 2 {
            return Result::Err(m());
        }
        let base = nonneg(&a[0]).ok_or_else(m)?;
        let size = nonneg(&a[1]).ok_or_else(m)?;
        if base + size > mem_bytes {
            return Result::Err(Err::new("layout"));
        }
        if let Some(bb) = block_base {
            if base + size > bb {
                return Result::Err(Err::new("layout"));
            }
        }
        scratch = Some((base, size));
    }
    // the modules
    let list = match obj.get("modules").and_then(|v| v.as_array()) {
        Some(l) if !l.is_empty() => l,
        _ => return Result::Err(Err::new("module_entry")),
    };
    let mut modules: Vec<Entry> = Vec::new();
    for e in list {
        let eo = e.as_object().ok_or_else(|| Err::new("module_entry"))?;
        let name = match eo.get("name").and_then(|n| n.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => return Result::Err(Err::new("module_entry")),
        };
        let named = |class: &'static str| Err::named(class, &name);
        let data_base = match eo.get("data_base") {
            None => return Result::Err(named("module_entry")),
            Some(v) => nonneg(v).ok_or_else(|| Err::new("manifest"))?,
        };
        let table_base = match eo.get("table_base") {
            None => return Result::Err(named("module_entry")),
            Some(v) => nonneg(v).ok_or_else(|| Err::new("manifest"))?,
        };
        let exports = match eo.get("exports") {
            None => Vec::new(),
            Some(v) => names(v).ok_or_else(|| named("module_entry"))?,
        };
        let data = match eo.get("data") {
            None => Vec::new(),
            Some(v) => names(v).ok_or_else(|| named("module_entry"))?,
        };
        if modules.iter().any(|p| p.name == name) {
            return Result::Err(named("module_entry"));
        }
        let fuel_budget = eo.get("fuel_budget").and_then(nonneg).map(|x| x as i64);
        modules.push(Entry { name, data_base, table_base, exports, data, fuel_budget, raw: e.clone() });
    }
    // each module: its window, its reach and its overlaps
    for (i, a) in modules.iter().enumerate() {
        if a.data_base + window > mem_bytes || a.table_base + table_slots > table_size {
            return Result::Err(Err::named("window", &a.name));
        }
        let end = a.data_base + window;
        if let Some(bb) = block_base {
            if end > bb {
                return Result::Err(Err::named("window_reaches", &a.name));
            }
        }
        if let Some((sb, _)) = scratch {
            if end > sb {
                return Result::Err(Err::named("window_reaches", &a.name));
            }
        }
        for b in &modules[i + 1..] {
            let ov = |x: u64, y: u64, w: u64| x < y + w && y < x + w;
            if ov(a.data_base, b.data_base, window) || ov(a.table_base, b.table_base, table_slots) {
                return Result::Err(Err { class: "overlap", modules: vec![a.name.clone(), b.name.clone()] });
            }
        }
    }
    Ok(Manifest {
        raw: raw.clone(),
        memory_pages,
        table_size,
        window,
        table_slots,
        block_base,
        block_size,
        block_max,
        scratch,
        modules,
    })
}

pub struct Loaded {
    pub instance: wasmi::Instance,
    pub data_words: Vec<(String, u32)>,
    pub budget: Option<i64>,
    pub fuel_pending: bool,
}

pub struct Loader<T> {
    pub engine: Engine,
    pub store: Store<T>,
    pub manifest: Option<Manifest>,
    pub memory: Option<Memory>,
    pub table: Option<Table>,
    pub modules: HashMap<String, Loaded>,
}

pub fn classify(e: &wasmi::Error) -> &'static str {
    match e.as_trap_code() {
        Some(TrapCode::UnreachableCodeReached) => "unreachable",
        Some(TrapCode::MemoryOutOfBounds) => "memory_access",
        Some(TrapCode::IntegerDivisionByZero) | Some(TrapCode::IntegerOverflow) | Some(TrapCode::BadConversionToInteger) => "divide",
        Some(TrapCode::TableOutOfBounds) | Some(TrapCode::IndirectCallToNull) | Some(TrapCode::BadSignature) => "table",
        Some(TrapCode::StackOverflow) => "stack",
        Some(TrapCode::OutOfFuel) => "out_of_fuel",
        _ => "unreachable",
    }
}

impl<T: 'static> Loader<T> {
    pub fn new(state: T) -> Loader<T> {
        let cfg = wasmi::Config::default();
        let engine = Engine::new(&cfg);
        let store = Store::new(&engine, state);
        Loader { engine, store, manifest: None, memory: None, table: None, modules: HashMap::new() }
    }

    pub fn set_manifest(&mut self, text: &[u8]) -> Result<(), Err> {
        self.manifest = Some(parse_manifest(text)?);
        Ok(())
    }

    pub fn create_memory(&mut self) -> Result<(), Err> {
        let m = self.manifest.as_ref().ok_or_else(|| Err::new("manifest"))?;
        let pages = m.memory_pages as u32;
        let tsize = m.table_size as u32;
        let mem = Memory::new(&mut self.store, MemoryType::new(pages, Some(pages))).map_err(|_| Err::new("layout"))?;
        let table = Table::new(&mut self.store, TableType::new(RefType::Func, tsize, Some(tsize)), Ref::Func(wasmi::Nullable::Null))
            .map_err(|_| Err::new("layout"))?;
        self.memory = Some(mem);
        self.table = Some(table);
        Ok(())
    }

    /// Loads a module (section 5.1, step 4). `define` adds the host's imports to the linker.
    pub fn load(&mut self, name: &str, wasm: &[u8], define: impl FnOnce(&mut Linker<T>) -> Result<(), wasmi::Error>) -> Result<(), Err> {
        let manifest = self.manifest.as_ref().ok_or_else(|| Err::new("manifest"))?;
        let entry = match manifest.modules.iter().find(|e| e.name == name) {
            Some(e) => e,
            None => return Result::Err(Err::named("not_in_manifest", name)),
        };
        let (mem, table) = match (self.memory, self.table) {
            (Some(m), Some(t)) => (m, t),
            _ => return Result::Err(Err::named("no_memory", name)),
        };
        if self.modules.contains_key(name) {
            return Result::Err(Err::named("loaded_twice", name));
        }
        check_majors(manifest, entry)?;
        let window = manifest.window as usize;
        let (data_base, table_base) = (entry.data_base as usize, entry.table_base as usize);
        let exports = entry.exports.clone();
        let data = entry.data.clone();
        let budget = entry.fuel_budget;
        // zero the window
        {
            let bytes = mem.data_mut(&mut self.store);
            let end = (data_base + window).min(bytes.len());
            bytes[data_base.min(end)..end].fill(0);
        }
        let module = Module::new(&self.engine, wasm).map_err(|_| Err::named("compile", name))?;
        let mut linker = <Linker<T>>::new(&self.engine);
        linker.define("env", "memory", mem).map_err(|_| Err::named("instantiate", name))?;
        linker.define("env", "__indirect_function_table", table).map_err(|_| Err::named("instantiate", name))?;
        define(&mut linker).map_err(|_| Err::named("instantiate", name))?;
        let instance = linker
            .instantiate_and_start(&mut self.store, &module)
            .map_err(|_| Err::named("instantiate", name))?;
        // the slots fill the free entries of the module's range
        let mut funcs = Vec::new();
        for ex in &exports {
            match instance.get_export(&self.store, ex) {
                Some(Extern::Func(f)) => funcs.push(f),
                _ => return Result::Err(Err::named("export", name)),
            }
        }
        let mut data_words = Vec::new();
        for d in &data {
            match instance.get_export(&self.store, d) {
                Some(Extern::Global(g)) => match g.get(&self.store) {
                    Val::I32(a) => data_words.push((d.clone(), a as u32)),
                    _ => return Result::Err(Err::named("export", name)),
                },
                _ => return Result::Err(Err::named("export", name)),
            }
        }
        for (i, f) in funcs.iter().enumerate() {
            let idx = (table_base + i) as u64;
            if let Some(r) = table.get(&self.store, idx) {
                if r.is_null() {
                    let _ = table.set(&mut self.store, idx, Ref::Func(wasmi::Nullable::Val(*f)));
                }
            }
        }
        self.modules.insert(name.to_string(), Loaded { instance, data_words, budget, fuel_pending: true });
        Ok(())
    }

    pub fn tick(&mut self) {
        for m in self.modules.values_mut() {
            m.fuel_pending = true;
        }
    }

    pub fn fuel(&mut self, name: &str) -> Option<i64> {
        let m = self.modules.get(name)?;
        let f = m.instance.get_func(&self.store, "__fuel_get")?;
        let mut out = [Val::I64(0)];
        f.call(&mut self.store, &[], &mut out).ok()?;
        match out[0] {
            Val::I64(v) => Some(v),
            _ => None,
        }
    }

    /// Sets the fuel at the first outermost call of a tick, when the module has a budget.
    pub fn prepare_call(&mut self, name: &str) {
        let (pending, budget) = match self.modules.get(name) {
            Some(m) => (m.fuel_pending, m.budget),
            None => return,
        };
        if !pending {
            return;
        }
        if let Some(b) = budget {
            if let Some(f) = self.modules[name].instance.get_func(&self.store, "__fuel_set") {
                let _ = f.call(&mut self.store, &[Val::I64(b)], &mut []);
            }
        }
        if let Some(m) = self.modules.get_mut(name) {
            m.fuel_pending = false;
        }
    }

    /// The class of a trap of a call of module `name`: a trap with the counter below zero is out of fuel.
    pub fn trap_error(&mut self, name: &str, e: &wasmi::Error) -> Err {
        let mut class = classify(e);
        if class == "unreachable" {
            if let Some(f) = self.fuel(name) {
                if f < 0 {
                    class = "out_of_fuel";
                }
            }
        }
        Err::named(class, name)
    }

    pub fn call(&mut self, name: &str, export: &str, args: &[i32]) -> Result<Option<i32>, Err> {
        let func: Func = {
            let m = match self.modules.get(name) {
                Some(m) => m,
                None => return Result::Err(Err::named("not_in_manifest", name)),
            };
            match m.instance.get_export(&self.store, export) {
                Some(Extern::Func(f)) => f,
                _ => return Result::Err(Err::named("export", name)),
            }
        };
        self.prepare_call(name);
        let ty = func.ty(&self.store);
        let params: Vec<Val> = ty
            .params()
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let a = args.get(i).copied().unwrap_or(0);
                match p {
                    ValType::I32 => Val::I32(a),
                    ValType::I64 => Val::I64(a as i64),
                    ValType::F32 => Val::F32((a as f32).into()),
                    _ => Val::F64((a as f64).into()),
                }
            })
            .collect();
        let mut results: Vec<Val> = ty.results().iter().map(|_| Val::I32(0)).collect();
        match func.call(&mut self.store, &params, &mut results) {
            Ok(()) => Ok(match results.first() {
                Some(Val::I32(v)) => Some(*v),
                _ => None,
            }),
            Err(e) => Result::Err(self.trap_error(name, &e)),
        }
    }
}

fn check_majors(manifest: &Manifest, entry: &Entry) -> Result<(), Err> {
    let mine = match entry.raw.get("majors") {
        Some(Value::Object(o)) => o,
        _ => return Ok(()),
    };
    let theirs = manifest.raw.get("majors").and_then(|v| v.as_object());
    for (iface, m) in mine {
        let m = m.as_u64();
        let t = theirs.and_then(|o| o.get(iface)).and_then(|v| v.as_u64());
        match (m, t) {
            (Some(m), Some(t)) => {
                if m > t {
                    return Result::Err(Err::new("manifest"));
                }
                if m < t && !adapters_cover(manifest, entry, iface, m) {
                    return Result::Err(Err::new("manifest"));
                }
            }
            _ => return Result::Err(Err::new("manifest")),
        }
    }
    Ok(())
}

/// An older major needs an adapter for every slot or import whose signature differs from the manifest's.
fn adapters_cover(manifest: &Manifest, entry: &Entry, iface: &str, from_major: u64) -> bool {
    let sigs = manifest.raw.get("signatures").and_then(|v| v.get(iface));
    let adapters = manifest.raw.get("adapters").and_then(|v| v.as_array());
    let has = |name: &str| {
        adapters.map_or(false, |a| {
            a.iter().any(|x| {
                x.get("interface").and_then(|v| v.as_str()) == Some(iface)
                    && x.get("from_major").and_then(|v| v.as_u64()) == Some(from_major)
                    && x.get("name").and_then(|v| v.as_str()) == Some(name)
            })
        })
    };
    let mut theirs: Vec<(String, &Value)> = Vec::new();
    if entry.raw.get("interface").and_then(|v| v.as_str()) == Some(iface) {
        if let Some(Value::Object(s)) = entry.raw.get("slots") {
            theirs.extend(s.iter().map(|(k, v)| (k.clone(), v)));
        }
    }
    if let Some(Value::Object(imps)) = entry.raw.get("imports") {
        if let Some(Value::Object(g)) = imps.get(iface) {
            theirs.extend(g.iter().map(|(k, v)| (k.clone(), v)));
        }
    }
    for (name, sig) in theirs {
        let cur = sigs.and_then(|s| s.get(&name));
        if cur != Some(sig) && !has(&name) {
            return false;
        }
    }
    true
}

pub fn run(calls: &[Call]) -> Vec<String> {
    let mut l: Loader<()> = Loader::new(());
    let mut out = Vec::new();
    for c in calls {
        let a = &c.args;
        let v: Result<String, Err> = (|| match c.name.as_str() {
            "manifest" => {
                let text = std::fs::read(&a[0]).map_err(|_| Err::new("manifest"))?;
                l.set_manifest(&text)?;
                Ok("null".to_string())
            }
            "memory" => {
                l.create_memory()?;
                Ok("null".to_string())
            }
            "load" => {
                let wasm = std::fs::read(&a[1]).map_err(|_| Err::named("compile", &a[0]))?;
                l.load(&a[0], &wasm, |_| Ok(()))?;
                Ok("null".to_string())
            }
            "tick" => {
                l.tick();
                Ok("null".to_string())
            }
            "call" => {
                let args: Vec<i32> = a[2..].iter().map(|s| s.trim().parse::<i64>().unwrap_or(0) as i32).collect();
                let r = l.call(&a[0], &a[1], &args)?;
                Ok(match r {
                    Some(v) => format!("{{\"result\":{}}}", v),
                    None => "{\"result\":null}".to_string(),
                })
            }
            "fuel" => Ok(l.fuel(&a[0]).map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())),
            "word" => {
                let addr: usize = a[0].trim().parse().unwrap_or(0);
                let mem = l.memory.ok_or_else(|| Err::new("no_memory"))?;
                let bytes = mem.data(&l.store);
                if addr + 4 <= bytes.len() {
                    Ok(i32::from_le_bytes([bytes[addr], bytes[addr + 1], bytes[addr + 2], bytes[addr + 3]]).to_string())
                } else {
                    Ok("0".to_string())
                }
            }
            _ => Ok("null".to_string()),
        })();
        match v {
            Ok(s) => out.push(format!("{}\t{}", c.name, s)),
            Result::Err(e) => {
                out.push(format!("{}\t{}", c.name, e.json()));
                break;
            }
        }
    }
    out
}
