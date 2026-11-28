// Local replayer for the pathfinding recordings: node pfreplay.js module.wasm recording.txt
const fs = require('fs');
const [wasmPath, recPath] = process.argv.slice(2);
const BB = 0x100000, SCR = 0xF0000, PAGES = 4112;
const mem = new WebAssembly.Memory({ initial: PAGES, maximum: PAGES });
const table = new WebAssembly.Table({ element: 'anyfunc', initial: 528, maximum: 528 });
const u8 = new Uint8Array(mem.buffer);
const dv = new DataView(mem.buffer);
const GAME = BB + 0x100000; // block offset 1 MiB
let expected = null; // queue of imports for the current call
let cur = null;
const imps = {};
function mk(name) {
  return (...args) => {
    if (!cur) throw new Error('import outside a call: ' + name);
    const e = cur.imports.shift();
    if (!e) throw new Error(`call ${cur.n} (${cur.slot}) line ${cur.line}: extra import ${name} ${args}`);
    const want = e.name;
    let a = args.map(x => x >>> 0);
    if (name === 'UnitState') a = [a[0]];
    if (name === 'SeenSize') a = [];
    const sa = e.args.join(' ');
    if (want !== name || a.join(' ') !== sa) {
      throw new Error(`call ${cur.n} (${cur.slot}) line ${e.line}: import mismatch: want ${want} ${sa}, got ${name} ${a.join(' ')}`);
    }
    for (const o of e.outs) {
      if (name === 'UnitState') {
        const m = o.match(/^out \((-?\d+),\((-?\d+),(-?\d+)\),(\d+),(\d+),(-?\d+),(-?\d+)\)/);
        const p = args[1];
        dv.setInt32(p, +m[1], true); dv.setInt16(p + 4, +m[2], true); dv.setInt16(p + 6, +m[3], true);
        dv.setUint16(p + 8, +m[4], true); dv.setInt32(p + 12, +m[5], true); dv.setInt32(p + 16, +m[6], true); dv.setInt32(p + 20, +m[7], true);
      } else if (name === 'SeenSize') {
        const m = o.match(/^(width|height) (-?\d+)/);
        dv.setInt32(m[1] === 'width' ? args[0] : args[1], +m[2], true);
      }
    }
    return e.res | 0;
  };
}
for (const n of ['Tick','SearchStepLimit','PlayerCount','IsPlaying','UnitsPerPlayer','FirstUnit','LastUnit','NextUnit','UnitState','MoveClass','InBounds','Pass','SeenSize','Seen','LineOfSight','TerrainPass','BeginSearch','EndSearch']) imps[n] = mk(n);
const mod = new WebAssembly.Module(fs.readFileSync(wasmPath));
const inst = new WebAssembly.Instance(mod, { env: { memory: mem, __indirect_function_table: table }, pathworld: imps });
const ex = inst.exports;
const gw = n => ex[n].value;
function setWord(n, v) { dv.setUint32(gw(n), v, true); }
setWord('g_game', GAME); setWord('g_blockBase', BB); setWord('g_simGlobals', GAME + 0x20000);
// parse
const lines = fs.readFileSync(recPath, 'utf8').split('\n');
const calls = [];
let c = null;
lines.forEach((ln, i) => {
  if (ln.startsWith('call ')) { const p = ln.split(' '); c = { tick: +p[1], slot: p[2], line: i + 1, r: [], a: {}, imports: [], w: [], o: [] }; calls.push(c); return; }
  if (!c || !ln.startsWith('  ')) return;
  const t = ln.slice(2);
  const sp = t.indexOf(' ');
  const k = t.slice(0, sp), rest = t.slice(sp + 1);
  if (k === 'r') c.r.push(rest);
  else if (k === 'a') { const s2 = rest.indexOf(' '); c.a[rest.slice(0, s2)] = rest.slice(s2 + 1); }
  else if (k === 'w') c.w.push(rest);
  else if (k === 'i') {
    let body = rest, res = 0;
    const ix = body.indexOf(' = ');
    if (ix >= 0) { res = parseInt(body.slice(ix + 3)); body = body.slice(0, ix); }
    const toks = body.split(' ');
    c.imports.push({ name: toks[0].replace('pathworld.', ''), args: toks.slice(1), res, outs: [], line: i + 1 });
  } else if (k === 'o') {
    if (c.imports.length && !/^(result|out x:)/.test(rest) && !c.closing) c.imports[c.imports.length - 1].outs.push(rest);
    else c.o.push(rest);
  }
});
let region = 0, size = 0, shadow = null;
function runs(str, target, base) {
  for (const tok of str.split(' ')) {
    if (tok.startsWith('[')) continue;
    const m = tok.match(/^(\d+):([0-9a-f]*)$/);
    if (!m) continue;
    Buffer.from(m[2], 'hex').copy(target, +m[1]);
  }
}
function applyRegion(str, toMem) {
  const whole = str.match(/^\[(\d+)\]/);
  if (whole) { shadow = Buffer.alloc(+whole[1]); }
  runs(str, shadow, 0);
}
let fail = 0;
const only = process.env.ONLY ? +process.env.ONLY : -1;
for (let n = 0; n < calls.length; n++) {
  const k = calls[n];
  cur = { n, slot: k.slot, imports: k.imports.map(x => ({ ...x })), line: k.line };
  try {
    for (const r of k.r) {
      if (r.startsWith('Game.width')) dv.setInt32(GAME + 82483, parseInt(r.split(' ')[1]), true);
      else if (r.startsWith('Game.height')) dv.setInt32(GAME + 82487, parseInt(r.split(' ')[1]), true);
      else if (r.startsWith('region ')) {
        const body = r.slice(7);
        runs(body, shadow, 0);
        runs(body, u8.subarray(region, region + size), 0);
        // runs() copies into a Buffer; for the memory view use a Buffer wrapper
        const wb = Buffer.from(mem.buffer, region, size);
        runs(body, wb, 0);
      }
    }
    u8.fill(0xAA, 327776, 331872);
    let res;
    if (k.slot === 'RegionSize') {
      const m = k.a.map.match(/\((\d+),(\d+),(\d+),(\d+)\)/);
      m.slice(1).forEach((v, i) => dv.setInt32(SCR + 4 * i, +v, true));
      res = ex.RegionSize(SCR) >>> 0;
    } else if (k.slot === 'Init') {
      const m = k.a.map.match(/\((\d+),(\d+),(\d+),(\d+)\)/);
      m.slice(1).forEach((v, i) => dv.setInt32(SCR + 4 * i, +v, true));
      size = +k.a.size; region = BB + +k.a.region;
      u8.fill(0, region, region + size);
      setWord('g_region', region);
      ex.Init(SCR, region, size);
    } else if (k.slot === 'Submit') {
      const a = k.a, p = SCR + 0x100;
      const pt = s => s.match(/-?\d+/g).map(Number);
      dv.setInt32(p, +a['request.unit'], true); dv.setUint32(p + 4, +a['request.notBefore'], true);
      dv.setInt32(p + 8, +a['request.goalKind'], true);
      const c2 = pt(a['request.goalCentre']); dv.setInt16(p + 12, c2[0], true); dv.setInt16(p + 14, c2[1], true);
      dv.setInt32(p + 16, +a['request.goalRadius0'], true); dv.setInt32(p + 20, +a['request.goalRadius1'], true);
      dv.setInt32(p + 24, +a['request.goalLeft'], true); dv.setInt32(p + 28, +a['request.goalRight'], true);
      dv.setInt32(p + 32, +a['request.goalTop'], true); dv.setInt32(p + 36, +a['request.goalBottom'], true);
      dv.setUint32(p + 40, +a['request.requester'], true); dv.setUint32(p + 44, +a['request.target'], true);
      ex.Submit(p);
    } else if (k.slot === 'Cancel') ex.Cancel(+k.a.requester);
    else if (k.slot === 'Step') ex.Step();
    else if (k.slot === 'Poll') {
      const out = SCR + 0x400, pts = SCR + 0x800;
      u8.fill(0, out, out + 64); u8.fill(0, pts, pts + 512);
      res = ex.Poll(out, +k.a.size, pts, +k.a.capacity);
      for (const o of k.o) {
        const m = o.match(/^out x:([0-9a-f]*)/);
        if (m) {
          const got = Buffer.from(mem.buffer, out, m[1].length / 2).toString('hex');
          if (got !== m[1]) throw new Error(`call ${n} Poll out: want ${m[1]} got ${got}`);
        }
      }
    } else if (k.slot === 'CellsChanged') ex.CellsChanged(0, 0, 0, 0, 0, 0);
    else throw new Error('slot ' + k.slot);
    for (const o of k.o) {
      const m = o.match(/^result (-?\d+)/);
      if (m && (res | 0) !== (+m[1] | 0)) throw new Error(`call ${n} (${k.slot}) result: want ${m[1]} got ${res}`);
    }
    if (cur.imports.length) throw new Error(`call ${n} (${k.slot}) line ${k.line}: ${cur.imports.length} imports not made, first ${cur.imports[0].name} ${cur.imports[0].args}`);
    { let lo=327776; while(lo<331872&&u8[lo]===0xAA)lo++; global.minLo=Math.min(global.minLo??1e9,lo); }
    for (const w of k.w) if (w.startsWith('region ')) applyRegion(w.slice(7));
    if (process.env.NOW) k.w.length;
    if (shadow) {
      const act = Buffer.from(mem.buffer, region, size);
      if (Buffer.compare(act, shadow) !== 0) {
        let i = 0; while (act[i] === shadow[i]) i++;
        throw new Error(`call ${n} tick ${k.tick} (${k.slot}) line ${k.line}: region byte ${i}: recorded ${shadow[i].toString(16)}, got ${act[i].toString(16)}`);
      }
    }
  } catch (e) {
    console.log('FAIL', e.message);
    fail = 1;
    break;
  }
}
console.log('stack used', 331872-global.minLo);
console.log(fail ? 'differs' : `equal (${calls.length} calls)`);
