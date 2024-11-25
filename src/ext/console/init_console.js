import * as _console from 'ext:deno_console/01_console.js';

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    console: nonEnumerable(
      new _console.Console((msg, level) => globalThis.Deno.core.print(msg, level > 1)),
    ),
});

globalThis.Deno.inspect = _console.inspect;