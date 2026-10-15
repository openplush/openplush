const p = require('./pkg/wasm_panic_probe.js');
function attempt(name, f) {
  try { console.log(`${name}: returned`, f()); }
  catch (e) { console.log(`${name}: threw ${e.constructor.name}: ${e.message}`); }
}
attempt('ok (before)', () => p.ok());
attempt('caught', () => p.caught());
attempt('ok (after caught)', () => p.ok());
attempt('boom_while_borrowed', () => p.boom_while_borrowed());
attempt('ok (after trap)', () => p.ok());
attempt('oom', () => p.oom());
attempt('recurse', () => p.recurse(0));
