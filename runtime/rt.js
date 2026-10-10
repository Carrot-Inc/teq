"use strict";
const $out = [];
let $outSize = 0;
const $hasProcess = typeof process !== "undefined" && process.stdout && typeof process.stdout.write === "function";
// `require` does not exist in an ES module; without fs the output goes through process.stdout.
let $fs = null;
if ($hasProcess) {
  try {
    $fs = typeof process.getBuiltinModule === "function" ? process.getBuiltinModule("fs") : require("fs");
  } catch (e) {}
}
// Buffering is only safe while main is the only thing that runs: an ES module and callbacks that
// fire after main share stdout with other JS code, so there every print is written right away.
let $sync = false;
function $flush() {
  if ($out.length === 0) return;
  const text = $out.join("");
  $out.length = 0;
  $outSize = 0;
  if ($hasProcess) {
    let written = false;
    if ($fs !== null) {
      try { $fs.writeSync(1, text); written = true; } catch (e) {}
    }
    if (!written) process.stdout.write(text);
  } else {
    console.log(text.endsWith("\n") ? text.slice(0, -1) : text);
  }
}
function $print(s) {
  $out.push(s);
  $outSize += s.length;
  // console.log cannot print a partial line, so without a process a line is completed first.
  if ($outSize > 65536 || ($sync && ($hasProcess || s.endsWith("\n")))) $flush();
}
function $println(s) { $print(s + "\n"); }
function $printTo(stream, text) {
  if (stream === "err") {
    if (typeof process !== "undefined" && process.stderr) process.stderr.write(text);
    else console.error(text.endsWith("\n") ? text.slice(0, -1) : text);
  } else $print(text);
}
// process.exit() called from inside main never reaches the flush at the end of $main.
if ($hasProcess && typeof process.on === "function") process.on("exit", $flush);
function $main(f) {
  try { f(); } finally { $flush(); $sync = true; }
}
function $args() { return $hasProcess && Array.isArray(process.argv) ? process.argv.slice(2) : []; }
// What the initialiser of a file's top-level vals rebinds itself to once it has started, so that
// the check in front of an access of the file's definitions is a call of nothing, which an
// engine inlines away where a test of a flag set at run time stays in a hot loop's code.
function $nop() {}
const $exc = {};
function $fail(n, m) { const c = $exc[n]; throw c ? new c(m) : new Error(n + ": " + m); }
function $matchError(v) { const c = $exc.MatchError; throw c ? new c(v) : new Error("MatchError: " + $str(v)); }
// `getClass.getName` of a value: the boxed class of a primitive, the qualified name of a class.
function $classOf(x) {
  switch (typeof x) {
    case "number": return (x | 0) === x ? "java.lang.Integer" : "java.lang.Double";
    case "string": return x.length === 1 ? "java.lang.Character" : "java.lang.String";
    case "boolean": return "java.lang.Boolean";
    case "bigint": return "java.lang.Long";
    case "undefined": return "scala.runtime.BoxedUnit";
    case "function": return x.$className === undefined ? "scala.Function1" : x.$className;
  }
  if (x === null) return "null";
  if (Array.isArray(x)) return "java.util.ArrayList";
  return x.$qname !== undefined ? x.$qname : (x.$className || "java.lang.Object");
}
// `$exc` holds the exception classes of the standard library that the runtime throws, set by
// their registration when the program has them: a program that catches nothing is spared them
// and gets plain errors. `$wrapJs` is what `catch` sees of a thrown value that is no Throwable
// (a non-local `return` on its way to its method passes every catch), `$unwrapJs` what `throw`
// throws of it.
function $wrapJs(e) {
  if (e instanceof $exc.Throwable) return e;
  if (e !== null && typeof e === "object" && e.$nlr !== undefined) throw e;
  return new $exc.JavaScriptException(e);
}
function $unwrapJs(e) { return e instanceof $exc.JavaScriptException ? e.exception : e; }
// A partial function is its own `apply`; the methods of the trait come from the prototype.
const $pfMissed = Symbol("missed");
const $pfMiss = () => $pfMissed;
let $pfProto = null;
function $pfInit(trait) {
  $pfProto = trait.prototype;
  Object.setPrototypeOf($pfProto, Function.prototype);
  // A library's call of the abstract `apply` (`pf.apply(x)`) is the call of the function.
  if (!Object.hasOwn($pfProto, "apply")) $pfProto.apply = function (x) { return this(x); };
}
function $pf(applyOrElse, isDefinedAt) {
  const f = (x) => applyOrElse(x, $matchError);
  f.applyOrElse = applyOrElse;
  f.isDefinedAt = isDefinedAt;
  return Object.setPrototypeOf(f, $pfProto);
}
function $idiv(a, b) {
  if (b === 0) $fail("ArithmeticException", "/ by zero");
  return (a / b) | 0;
}
function $irem(a, b) {
  if (b === 0) $fail("ArithmeticException", "/ by zero");
  return (a % b) | 0;
}
function $ldiv(a, b) {
  if (b === 0n) $fail("ArithmeticException", "/ by zero");
  return BigInt.asIntN(64, a / b);
}
function $lrem(a, b) {
  if (b === 0n) $fail("ArithmeticException", "/ by zero");
  return a % b;
}
function $lushr(a, b) { return BigInt.asIntN(64, BigInt.asUintN(64, a) >> (b & 63n)); }
function $d2i(d) {
  if (d !== d) return 0;
  if (d >= 2147483647) return 2147483647;
  if (d <= -2147483648) return -2147483648;
  return d | 0;
}
function $d2l(d) {
  if (d !== d) return 0n;
  if (d >= 9223372036854775807) return 9223372036854775807n;
  if (d <= -9223372036854775808) return -9223372036854775808n;
  return BigInt(Math.trunc(d));
}
function $isInt(x) { return typeof x === "number" && (x | 0) === x; }
function $isByte(x) { return typeof x === "number" && (x << 24 >> 24) === x; }
function $isShort(x) { return typeof x === "number" && (x << 16 >> 16) === x; }
function $isFloat(x) { return typeof x === "number" && (x !== x || Math.fround(x) === x); }
// `x.getClass`: one `Class` per JS class of a Scala value, made from the names the emitter
// registers on its prototype, or per primitive type.
const $primClasses = {};
// The simple name is the part after the last `.`, and after the last `$` of a class nested in
// another (`SchemaType$SCoproduct`), as the JVM's `getSimpleName` gives it.
function $simpleName(qname) {
  const name = qname.slice(qname.lastIndexOf(".") + 1);
  const at = name.lastIndexOf("$");
  return at >= 0 && at < name.length - 1 ? name.slice(at + 1) : name;
}
// A primitive's `Class` prints its bare name (`int`, `void`), as the JDK does; the others `class ` first.
const $primitiveClassNames = new Set(["boolean", "byte", "short", "char", "int", "long", "float", "double", "void"]);
function $makeClass(ctor, qname) {
  return { $ctor: ctor, $qname: qname, $className: $simpleName(qname), toString() { return $primitiveClassNames.has(qname) ? qname : "class " + qname; } };
}
function $getClass(x) {
  let qname;
  switch (typeof x) {
    // A boxed number has no width of its own on JavaScript: an integral one is an `Integer`, the
    // JVM's class of an `Int` (a boxed `Byte` or `Short` prints so too), a fractional one a `Double`.
    case "number": qname = Number.isInteger(x) ? "java.lang.Integer" : "java.lang.Double"; break;
    case "string": qname = "java.lang.String"; break;
    case "boolean": qname = "java.lang.Boolean"; break;
    case "bigint": qname = "java.lang.Long"; break;
    case "undefined": qname = "scala.runtime.BoxedUnit"; break;
    case "function": if (x.constructor === Function) qname = "scala.Function1"; break;
  }
  if (qname === undefined && Array.isArray(x)) qname = "[Ljava.lang.Object;";
  if (qname !== undefined) return $primClasses[qname] ??= $makeClass(null, qname);
  const c = x.constructor;
  // A plain object (`new Object()`) is of no Scala class.
  if (typeof c !== "function" || c.prototype.$qname === undefined) return $classNamed("java.lang.Object");
  return Object.prototype.hasOwnProperty.call(c, "$class") ? c.$class : (c.$class = $makeClass(c, c.prototype.$qname));
}
// `classOf[C]`: the `Class` of a JS class, or of a name the output has no class for.
function $classValue(c) {
  return Object.prototype.hasOwnProperty.call(c, "$class") ? c.$class : (c.$class = $makeClass(c, c.prototype.$qname));
}
function $classNamed(qname) { return $primClasses[qname] ??= $makeClass(null, qname); }
// The `Class` of a Scala class the output has no constructor for, with the qualified names of
// its ancestors, which `isAssignableFrom` reads; a module run again under --hot names them anew.
function $classData(qname, ancestors) {
  const c = $classNamed(qname);
  c.$ancestors = ancestors;
  return c;
}
// The classes and objects registered for reflective instantiation, by their binary names: a
// class with its constructors, each a function giving the classes of its parameters and a
// function from an array of arguments, an object with the accessor of its instance. A module run
// again registers anew.
const $reflectedClasses = new Map();
const $reflectedModules = new Map();
// An exception whose class takes no cause answers `getCause` with the one it is given.
function $withCause(e, cause) { e.getCause = () => cause; return e; }
// `initCause`: the cause the instance answers from now on, given once.
function $giveCause(e, cause) { e.$causeGiven = true; e.getCause = () => cause; }
// `addSuppressed` and `getSuppressed`: the suppressed exceptions, kept on the exception from the first;
// `null` once its constructor disabled suppression, which records none.
function $suppress(e, s) { if (e.$suppressed !== null) (e.$suppressed ??= []).push(s); }
function $suppressed(e) { return e.$suppressed == null ? [] : e.$suppressed.slice(); }
// Under --hot each module's registrations run in its scope, which it clears when it runs again.
let $reflectOwner;
function $reflectScope(module) {
  $reflectOwner = module;
  for (const registry of [$reflectedClasses, $reflectedModules]) {
    for (const [name, entry] of registry) if (entry.owner === module) registry.delete(name);
  }
}
function $reflectKeep(modules) {
  for (const registry of [$reflectedClasses, $reflectedModules]) {
    for (const [name, entry] of registry) if (!modules.includes(entry.owner)) registry.delete(name);
  }
}
function $reflectClass(name, cls, ctors) { $reflectedClasses.set(name, { cls, ctors, owner: $reflectOwner }); }
function $reflectModule(name, cls, load) { $reflectedModules.set(name, { cls, load, owner: $reflectOwner }); }
// An instance of a class that is a function: the function takes the class's prototype, and the
// members named like a function's own `length` and `name` are put on it in their place.
function $getChars(s, from, to, dst, at) {
  for (let i = from; i < to; i++) dst[at + i - from] = s[i];
}
function $callable(f, proto) {
  Object.setPrototypeOf(f, proto);
  for (const k of ["length", "name"]) if (typeof proto[k] === "function") Object.defineProperty(f, k, { value: proto[k], writable: true, configurable: true });
}
// Every class but a primitive's is below `Object`; a trait's `Class` knows its number, which the
// prototypes of the classes and traits that take it carry (`$cls`).
function $isAssignableFrom(a, b) {
  if (a.$qname === "java.lang.Object") return !["int", "long", "double", "float", "short", "byte", "char", "boolean", "void"].includes(b.$qname);
  return a === b || (a.$ctor !== null && b.$ctor !== null && b.$ctor.prototype instanceof a.$ctor)
    || (a.$trait !== undefined && b.$ctor !== null && b.$ctor.prototype["$i" + a.$trait] === true)
    || (b.$ancestors !== undefined && b.$ancestors.includes(a.$qname));
}
// `classOf[T]` of a trait: its `Class` with its number for `isAssignableFrom`.
function $traitClass(c, n) {
  const k = $classValue(c);
  k.$trait = n;
  return k;
}
// The `Class` of a class a primitive's box extends (`CharSequence` a string's, `Number` a
// number's): the kinds of primitive, a bit set of `$boxedKind`'s, that `isInstance` takes.
function $boxedClass(k, boxed) {
  k.$boxed = boxed;
  return k;
}
function $boxedKind(x) {
  switch (typeof x) {
    case "string": return 1;
    case "number": return 2;
    case "bigint": return 4;
    case "boolean": return 8;
    case "undefined": return 32;
  }
  return 0;
}
function $classOfObject() { return $classNamed("java.lang.Object"); }
// `Class.isInstance`: a primitive is an instance of its box and of what its box extends; `null` is no instance; a
// trait's `Class` tests its number (`$isA`), as a type test against the trait does; a class with a
// constructor tests it, and another its `Class`.
function $isInstance(cls, x) {
  if ((cls.$boxed & $boxedKind(x)) !== 0) return true;
  if (x == null) return false;
  if (typeof x !== "object" && typeof x !== "function") return $getClass(x).$qname === cls.$qname;
  if (cls.$trait !== undefined) return $isA(x, cls.$trait);
  return cls.$ctor !== null ? x instanceof cls.$ctor : $getClass(x) === cls;
}
function $isRef(x) { return x !== undefined && typeof x !== "number" && typeof x !== "boolean" && typeof x !== "bigint"; }
function $isA(x, id) { return x != null && x["$i" + id] === true; }
// `x.asInstanceOf[T]`: `null` and a value that passes the test of `T`'s erasure are the value, any
// other fails with the JVM's message, the value's class and the target's named. A class is tested
// by `instanceof`, a trait by its number as a type test of it is (`$isA`, which `()`, `undefined`,
// fails), a `String` by its JS type, any other type by the test `ok` written at the cast.
function $cce(x, to) { $fail("ClassCastException", "class " + $classOf(x) + " cannot be cast to class " + to); }
function $as(x, c) { return x === null || x instanceof c ? x : $cce(x, $classOf(c.prototype)); }
function $asA(x, n, to) { return x === null || $isA(x, n) ? x : $cce(x, to); }
function $asT(x, ok, to) { return x === null || ok ? x : $cce(x, to); }
function $asS(x) { return x === null || typeof x === "string" ? x : $cce(x, "java.lang.String"); }
function $asNothing(x) { $fail("ClassCastException", "Cannot cast to scala.Nothing"); }
// The unboxings of `x.asInstanceOf[Int]` and kin: `null` is the primitive's zero, a value of
// another kind fails (`BoxesRunTime.unboxToInt`).
function $uI(x) { return x === null ? 0 : $isInt(x) ? x : $cce(x, "java.lang.Integer"); }
function $uJ(x) { return x === null ? 0n : typeof x === "bigint" ? x : $cce(x, "java.lang.Long"); }
function $uD(x) { return x === null ? 0 : typeof x === "number" ? x : $cce(x, "java.lang.Double"); }
function $uF(x) { return x === null ? 0 : $isFloat(x) ? x : $cce(x, "java.lang.Float"); }
function $uB(x) { return x === null ? 0 : $isByte(x) ? x : $cce(x, "java.lang.Byte"); }
function $uS(x) { return x === null ? 0 : $isShort(x) ? x : $cce(x, "java.lang.Short"); }
function $uC(x) { return x === null ? "\0" : typeof x === "string" ? x : $cce(x, "java.lang.Character"); }
function $uZ(x) { return x === null ? false : typeof x === "boolean" ? x : $cce(x, "java.lang.Boolean"); }
// Doubles print as Scala.js prints them: the JS number format.
function $dstr(d) { return String(d); }
function $str(x) {
  if (x === undefined) return "()";
  if (x === null) return "null";
  switch (typeof x) {
    case "string": return x;
    case "number": case "boolean": case "bigint": return String(x);
    case "function": if (x.$className === undefined) return "<function>"; break;
  }
  if (Array.isArray(x)) return "Array(" + x.map($str).join(", ") + ")";
  if (x.toString !== Object.prototype.toString) return x.toString();
  return (x.$className || "Object") + "@" + $hash(x).toString(16);
}
// The program's `x.toString`, a call, which `null` fails as any member selected from it does.
function $toStr(x) { return x === null ? x.toString() : $str(x); }
// What `println` and `print` show of a value: the unit as Scala.js prints it, `undefined`, which a
// concatenation and `toString` render as `()`.
function $printed(x) { return x === undefined ? "undefined" : $str(x); }
// java.lang.Double.equals compares the bits: NaN equals NaN, 0.0 does not equal -0.0.
function $doubleEquals(a, b) { return a === b ? a !== 0 || 1 / a === 1 / b : a !== a && b !== b; }
// `x.equals(y)`, the method rather than `==`: a number equals a number of its bits (the boxes of
// Scala.js are the numbers, `Long` apart), a Long a Long, an object answers by its own `equals`
// (a null receiver fails as any member read of it does).
function $equals(a, b) {
  switch (typeof a) {
    case "number": return typeof b === "number" && $doubleEquals(a, b);
    case "bigint": return a === b;
    case "object": case "function": if (typeof a.equals === "function") return a.equals(b);
  }
  return a === b;
}
function $eq(a, b) {
  if (a === b) return true;
  if (a == null || b == null) return false;
  if ((typeof a === "object" || typeof a === "function") && typeof a.equals === "function") return a.equals(b);
  if (typeof a === "number" && typeof b === "bigint") return Number.isInteger(a) && BigInt(a) === b;
  if (typeof a === "bigint" && typeof b === "number") return Number.isInteger(b) && a === BigInt(b);
  return false;
}
let $nextHash = 1;
const $identityHashes = new WeakMap();
function $strHash(s) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (Math.imul(h, 31) + s.charCodeAt(i)) | 0;
  return h;
}
function $identityHash(x) {
  let h = $identityHashes.get(x);
  if (h === undefined) {
    h = $nextHash++;
    $identityHashes.set(x, h);
  }
  return h;
}
// `toString` of `Any`, which `super.toString` reaches where no class or trait defines one.
function $anyStr(x) { return (x.$className || "Object") + "@" + $identityHash(x).toString(16); }
// `Object.clone`: a shallow copy of the same class.
function $cloneObject(x) { return Object.assign(Object.create(Object.getPrototypeOf(x)), x); }
function $throw(e) { throw e; }
function $ref(get, set) { return { get v() { return get(); }, set v(x) { set(x); } }; }
function $enumValueOf(values, name, enumName) {
  for (const v of values) if (v.$name === name) return v;
  $fail("IllegalArgumentException", "enum " + enumName + " has no case with name: " + name);
}
function $enumFromOrdinal(values, ordinal, enumName) {
  if (ordinal < 0 || ordinal >= values.length) $fail("NoSuchElementException", "enum " + enumName + " has no case with ordinal: " + ordinal);
  return values[ordinal];
}
function $productPrefix(x) { return x.$name !== undefined ? x.$name : x.$className; }
// ---- scala.util.hashing.MurmurHash3 ----
function $mix(h, k) {
  k = Math.imul(k, 0xcc9e2d51);
  k = (k << 15) | (k >>> 17);
  k = Math.imul(k, 0x1b873593);
  h ^= k;
  h = (h << 13) | (h >>> 19);
  return (Math.imul(h, 5) + 0xe6546b64) | 0;
}
function $mixLast(h, k) {
  k = Math.imul(k, 0xcc9e2d51);
  k = (k << 15) | (k >>> 17);
  k = Math.imul(k, 0x1b873593);
  return h ^ k;
}
function $avalanche(h) {
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  return h ^ (h >>> 16);
}
function $finalizeHash(h, length) { return $avalanche(h ^ length); }
const $productSeed = 0xcafebabe | 0;
const $seqSeed = $strHash("Seq");
const $setSeed = $strHash("Set");
const $mapSeed = $strHash("Map");
const $tuple2Prefix = $strHash("Tuple2");
const $f64 = new Float64Array(1);
const $u32 = new Uint32Array($f64.buffer);
function $bitsHash(d) {
  if (d !== d) return 2146959360;
  $f64[0] = d;
  return ($u32[0] ^ $u32[1]) | 0;
}
// Statics.doubleHash of Scala.js for a number that is no Int: a whole number hashes as the Long
// it is, anything else by its bits.
function $doubleHash(d) {
  if (Number.isInteger(d) && Math.abs(d) < 9223372036854775808) {
    const hi = Math.floor(d / 4294967296);
    return (hi ^ (d - hi * 4294967296)) | 0;
  }
  return $bitsHash(d);
}
function $longHashCode(x) { return Number(BigInt.asIntN(32, x ^ (x >> 32n))); }
function $longHash(x) {
  const i = Number(BigInt.asIntN(32, x));
  return BigInt(i) === x ? i : $longHashCode(x);
}
// `##`, and the hash every collection and case class uses for its elements.
function $hash(x) {
  if (x == null) return 0;
  switch (typeof x) {
    case "number": { const i = x | 0; return i === x ? i : $doubleHash(x); }
    case "string": return $strHash(x);
    case "boolean": return x ? 1231 : 1237;
    case "bigint": return $longHash(x);
    case "object": case "function":
      if (typeof x.hashCode === "function") return x.hashCode();
  }
  let h = $identityHashes.get(x);
  if (h === undefined) {
    h = $nextHash++;
    $identityHashes.set(x, h);
  }
  return h;
}
// `x.hashCode`: differs from `##` for a Long outside the Int range and for a Double that is no
// Int, where Scala.js takes the bits, so -0.0 and NaN included.
function $hashCode(x) {
  if (typeof x === "number") {
    const i = x | 0;
    return i === x && (i !== 0 || 1 / x > 0) ? i : $bitsHash(x);
  }
  if (typeof x === "bigint") return $longHashCode(x);
  return $hash(x);
}
function $seqHash(xs) {
  let h = $seqSeed, n = 0, prev = 0, first = 0, step = 0, state = 0;
  xs.foreach((x) => {
    const k = $hash(x);
    h = $mix(h, k);
    if (state === 0) {
      first = k;
      state = 1;
    } else if (state === 1) {
      step = (k - prev) | 0;
      state = 2;
    } else if (state === 2 && step !== ((k - prev) | 0)) state = 3;
    prev = k;
    n++;
  });
  // Elements whose hashes form an arithmetic progression hash as the Range they could be.
  return state === 2 ? $avalanche($mix($mix($mix($seqSeed, first), step), prev)) : $finalizeHash(h, n);
}
function $unorderedHash(a, b, c, n, seed) {
  let h = $mix(seed, a);
  h = $mix(h, b);
  h = $mixLast(h, c);
  return $finalizeHash(h, n);
}
function $setHash(xs) {
  let a = 0, b = 0, c = 1, n = 0;
  xs.foreach((x) => {
    const k = $hash(x);
    a = (a + k) | 0;
    b ^= k;
    c = Math.imul(c, k | 1);
    n++;
  });
  return $unorderedHash(a, b, c, n, $setSeed);
}
function $caseToString() {
  const fields = this.$fields;
  let s = this.$className + "(";
  for (let i = 0; i < fields.length; i++) {
    if (i > 0) s += ",";
    s += $str(this[fields[i]]);
  }
  return s + ")";
}
function $caseEquals(that) {
  if (this === that) return true;
  if (that == null || that.constructor !== this.constructor) return false;
  const fields = this.$fields;
  for (let i = 0; i < fields.length; i++) {
    if (!$eq(this[fields[i]], that[fields[i]])) return false;
  }
  return true;
}
// `equals` of a case class that is extended: an instance of a subclass can be the equal one.
function $subEquals(c) {
  return function (that) {
    if (this === that) return true;
    if (!(that instanceof c)) return false;
    for (const f of this.$fields) if (!$eq(this[f], that[f])) return false;
    return true;
  };
}
// MurmurHash3.productHash; tuples are registered without a name.
function $caseHash() {
  const fields = this.$fields;
  const prefix = this.$className === "" ? "Tuple" + fields.length : this.$className;
  if (fields.length === 0) return $strHash(prefix);
  let h = $mix($productSeed, $strHash(prefix));
  for (let i = 0; i < fields.length; i++) h = $mix(h, $hash(this[fields[i]]));
  return $finalizeHash(h, fields.length);
}
function $nameHash() { return $strHash(this.$name); }
// The first n elements of a sequence as an array, with the rest of the sequence as element n
// when `rest` is set; null when the sequence is shorter, or longer without `rest`.
function $seqPat(x, n, rest, arraySeq) {
  if (Array.isArray(x)) {
    if (rest ? x.length < n : x.length !== n) return null;
    if (!rest) return x;
    const out = x.slice(0, n);
    out.push(new arraySeq(x.slice(n)));
    return out;
  }
  const out = [];
  const it = x.iterator();
  for (let i = 0; i < n; i++) {
    if (!it.hasNext()) return null;
    out.push(it.next());
  }
  if (rest) out.push(x.drop(n));
  else if (it.hasNext()) return null;
  return out;
}
// Makes everything with a foreach method usable in for-of, spreads and Array.from.
function $iterator() {
  const items = [];
  this.foreach((x) => { items.push(x); });
  return items[Symbol.iterator]();
}
// Registers a class: its name; its shape, the field names of a case class, 2 for a case object,
// 3 for an enum value, 4 for a trait or enum; the traits and enums it takes members from, in
// linearisation order (the first declaration wins); their numbers for type tests; an enum ordinal.
// The methods of a shape come by prototype, or copied below a superclass, which scalac lets win.
function $cls(c, name, shape, traits, ids, ordinal) {
  const p = c.prototype;
  if (shape === 4) p.$own = Object.getOwnPropertyDescriptors(p);
  p.$className = name;
  if (ids !== undefined) for (const id of ids) p["$i" + id] = true;
  if (traits !== undefined) {
    for (const t of traits) {
      const own = t.prototype.$own || Object.getOwnPropertyDescriptors(t.prototype);
      for (const key in own) {
        if (key !== "constructor" && !Object.prototype.hasOwnProperty.call(p, key)) Object.defineProperty(p, key, own[key]);
      }
    }
  }
  if (typeof p.foreach === "function") p[Symbol.iterator] = $iterator;
  if (!shape || shape === 4) return;
  if (shape !== 3) p.$fields = shape === 2 ? [] : shape;
  const base = shape === 3 ? $enumValueProto : shape === 2 ? $caseObjectProto : $caseProto;
  if (Object.getPrototypeOf(p) === Object.prototype) Object.setPrototypeOf(p, base);
  else for (const key in base) if (!Object.prototype.hasOwnProperty.call(p, key)) p[key] = base[key];
  if (ordinal !== undefined) p.$ordinal = ordinal;
}
const $caseProto = { toString: $caseToString, equals: $caseEquals, hashCode: $caseHash };
// The members of Product on a case class, a case object, an enum case or a tuple, which the
// registration above describes, or on a class that implements them (its own productElement first).
function $productArity(p) { return p.$fields !== undefined ? p.$fields.length : typeof p.productArity === "function" ? p.productArity() : 0; }
// A case class's names are on its prototype once the program asks for one (`$names`); a class
// that defines the method answers itself; any other product's names are empty.
function $productElementName(p, i) {
  const names = p.$names;
  if (names === undefined && typeof p.productElementName === "function") return p.productElementName(i);
  const arity = names !== undefined ? names.length : $productArity(p);
  if (i < 0 || i >= arity) $fail("IndexOutOfBoundsException", names !== undefined ? "" + i : i + " is out of bounds (min 0, max " + (arity - 1) + ")");
  return names !== undefined ? names[i] : "";
}
// A member of an object by its name, as a reflective structural call reaches it: a method is
// called, a field read. A name no member has is a TypeError, as under Scala.js.
function $memberByName(x, name) {
  const m = x[name];
  if (m === undefined) throw new TypeError(name + " is not a member of " + $str(x));
  return typeof m === "function" ? m.call(x) : m;
}
function $callByName(x, name, args) {
  const m = x[name];
  if (typeof m !== "function") throw new TypeError(name + " is not a method of " + $str(x));
  return m.apply(x, args);
}
function $productElement(p, i) {
  if (typeof p.productElement === "function") return p.productElement(i);
  const f = p.$fields;
  if (f === undefined) $fail("IndexOutOfBoundsException", "" + i);
  if (i < 0 || i >= f.length) $fail("IndexOutOfBoundsException", "" + i);
  return p[f[i]];
}
// A case object equals itself alone: one nested in a class has an instance per enclosing instance.
const $caseObjectProto = { __proto__: $caseProto, toString() { return this.$className; }, equals(that) { return this === that; } };
// `scala.reflect.Enum.ordinal`, an abstract member: the receiver's own implementation, else the one
// `DesugarEnums` gives an enum case, its `$ordinal`; `null` fails as a member selected from it does.
function $enumOrdinal(e) { return typeof e.ordinal === "function" ? e.ordinal() : e.$ordinal; }
const $enumValueProto = { toString() { return this.$name; }, hashCode: $nameHash };
// A module of the split output cannot name a superclass of another module while it loads.
function $ext(c, parent) {
  Object.setPrototypeOf(c, parent);
  Object.setPrototypeOf(c.prototype, parent.prototype);
}
function $max(a, b) { return typeof a === "bigint" ? (a > b ? a : b) : Math.max(a, b); }
function $min(a, b) { return typeof a === "bigint" ? (a < b ? a : b) : Math.min(a, b); }
function $numberFormat(s) { $fail("NumberFormatException", "For input string: \"" + s + "\""); }
function $intOrNull(s) {
  if (!/^[+-]?\d+$/.test(s)) return null;
  const n = Number(s);
  return n > 2147483647 || n < -2147483648 ? null : n;
}
function $longOrNull(s) {
  if (!/^[+-]?\d+$/.test(s)) return null;
  const n = BigInt(s);
  return n > 9223372036854775807n || n < -9223372036854775808n ? null : n;
}
// The decimal grammar of java.lang.Double.parseDouble, which is narrower than Number().
function $doubleOrNull(s) {
  const m = /^[\x00-\x20]*([+-]?)(?:(NaN|Infinity)|((?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)[fFdD]?)[\x00-\x20]*$/.exec(s);
  if (m === null) return null;
  const n = Number(m[2] ?? m[3]);
  return m[1] === "-" ? -n : n;
}
function $parseInt(s) { return $intOrNull(s) ?? $numberFormat(s); }
function $parseLong(s) { return $longOrNull(s) ?? $numberFormat(s); }
function $parseDouble(s) { return $doubleOrNull(s) ?? $numberFormat(s); }
function $parseIntRadix(s, radix) {
  const digits = "0123456789abcdefghijklmnopqrstuvwxyz".slice(0, radix);
  if (!new RegExp("^[+-]?[" + digits + "]+$", "i").test(s)) $numberFormat(s);
  const n = parseInt(s, radix);
  return n > 2147483647 || n < -2147483648 ? $numberFormat(s) : n;
}
function $arrayFill(n, thunk) {
  const a = new Array(n);
  for (let i = 0; i < n; i++) a[i] = thunk();
  return a;
}
function $arrayTabulate(n, f) {
  const a = new Array(n);
  for (let i = 0; i < n; i++) a[i] = f(i);
  return a;
}
// Int.MinValue stays itself as on the JVM; the Double -2147483648.0 is taken for it.
function $abs(x) {
  if (typeof x === "bigint") return BigInt.asIntN(64, x < 0n ? -x : x);
  return x <= 0 && x !== -2147483648 ? 0 - x : x;
}
// Math.rint: the nearest whole number, halves to the even one.
function $rint(x) {
  const r = Math.round(x);
  return r - x === 0.5 && r % 2 !== 0 ? r - 1 : r;
}
function $signum(x) { return typeof x === "bigint" ? (x > 0n ? 1n : x < 0n ? -1n : 0n) : x > 0 ? 1 : x < 0 ? -1 : x; }
function $floorMod(a, b) {
  const zero = typeof a === "bigint" ? 0n : 0;
  if (b === zero) $fail("ArithmeticException", "/ by zero");
  const r = a % b;
  if (r === zero) return zero;
  return (r < zero) !== (b < zero) ? r + b : r;
}
function $floorDiv(a, b) {
  const m = $floorMod(a, b);
  return typeof a === "bigint" ? BigInt.asIntN(64, (a - m) / b) : ((a - m) / b) | 0;
}
function $compareStrings(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const d = a.charCodeAt(i) - b.charCodeAt(i);
    if (d !== 0) return d;
  }
  return a.length - b.length;
}
function $compareTo(x, y) {
  switch (typeof x) {
    case "string": return $compareStrings(x, y);
    case "number": return $compareDoubles(x, y);
    case "bigint": case "boolean": return x === y ? 0 : x < y ? -1 : 1;
  }
  return x.compareTo(y);
}
// ---- java.lang and java.util statics of the platform layer (std/javalib) ----
function $bitCount(i) {
  i = i - ((i >>> 1) & 0x55555555);
  i = (i & 0x33333333) + ((i >>> 2) & 0x33333333);
  return (Math.imul((i + (i >>> 4)) & 0x0f0f0f0f, 0x01010101) >>> 24);
}
function $reverseBits(i) {
  i = ((i & 0x55555555) << 1) | ((i >>> 1) & 0x55555555);
  i = ((i & 0x33333333) << 2) | ((i >>> 2) & 0x33333333);
  i = ((i & 0x0f0f0f0f) << 4) | ((i >>> 4) & 0x0f0f0f0f);
  return ((i << 24) | ((i & 0xff00) << 8) | ((i >>> 8) & 0xff00) | (i >>> 24)) | 0;
}
function $toLong(n) { return BigInt(Math.trunc(n)); }
function $longToInt(l) { return Number(BigInt.asIntN(32, l)); }
function $parseLongUnchecked(s) { return $parseLong(s.trim()); }
function $parseDoubleUnchecked(s) { return $parseDouble(s.trim()); }
function $longBitCount(l) {
  const u = BigInt.asUintN(64, l);
  return $bitCount(Number(u & 0xffffffffn)) + $bitCount(Number(u >> 32n));
}
function $longClz(l) {
  const u = BigInt.asUintN(64, l);
  const hi = Number(u >> 32n);
  return hi !== 0 ? Math.clz32(hi) : 32 + Math.clz32(Number(u & 0xffffffffn));
}
function $longCtz(l) {
  const u = BigInt.asUintN(64, l);
  if (u === 0n) return 64;
  let n = 0;
  let v = u;
  while ((v & 1n) === 0n) { v >>= 1n; n++; }
  return n;
}
const $f64view = new DataView(new ArrayBuffer(8));
// Every NaN has Java's one pattern, as Scala.js gives it; the raw bits keep what the engine holds.
function $doubleToLongBits(d) {
  return d !== d ? 0x7ff8000000000000n : $doubleToRawLongBits(d);
}
function $doubleToRawLongBits(d) {
  $f64view.setFloat64(0, d);
  return $f64view.getBigInt64(0);
}
function $longBitsToDouble(l) {
  $f64view.setBigInt64(0, BigInt.asIntN(64, l));
  return $f64view.getFloat64(0);
}
// Character.toUpperCase and toLowerCase of the JDK on one UTF-16 unit: the simple case
// mapping, which the dotted capital I takes to a plain i; another mapping that takes several
// units leaves the unit as it is.
function $charUpper(c) {
  const u = String.fromCharCode(c).toUpperCase();
  return u.length === 1 ? u.charCodeAt(0) : c;
}
function $charLower(c) {
  if (c === 0x130) return 0x69;
  const l = String.fromCharCode(c).toLowerCase();
  return l.length === 1 ? l.charCodeAt(0) : c;
}
// String.regionMatches of the JDK, which compares unit by unit through the two case mappings
// when it ignores case.
function $regionMatches(s, ignoreCase, toffset, other, ooffset, len) {
  if (toffset < 0 || ooffset < 0 || toffset + len > s.length || ooffset + len > other.length) return false;
  for (let i = 0; i < len; i++) {
    const a = s.charCodeAt(toffset + i), b = other.charCodeAt(ooffset + i);
    if (a === b) continue;
    if (!ignoreCase) return false;
    const ua = $charUpper(a), ub = $charUpper(b);
    if (ua === ub) continue;
    if ($charLower(ua) !== $charLower(ub)) return false;
  }
  return true;
}
// String.equalsIgnoreCase of the JDK.
function $equalsIgnoreCase(s, other) {
  return other != null && s.length === other.length && $regionMatches(s, true, 0, other, 0, s.length);
}
// Math.nextUp and nextDown of the JDK: the adjacent double, through its bits.
function $nextUp(d) {
  if (d !== d || d === Infinity) return d;
  if (d === 0) return 4.9e-324;
  const bits = $doubleToLongBits(d);
  return $longBitsToDouble(d > 0 ? bits + 1n : bits - 1n);
}
function $nextDown(d) {
  if (d !== d || d === -Infinity) return d;
  if (d === 0) return -4.9e-324;
  const bits = $doubleToLongBits(d);
  return $longBitsToDouble(d > 0 ? bits - 1n : bits + 1n);
}
function $floatToIntBits(f) {
  if (f !== f) return 0x7fc00000;
  $f64view.setFloat32(0, f);
  return $f64view.getInt32(0);
}
function $intBitsToFloat(i) {
  $f64view.setInt32(0, i);
  return $f64view.getFloat32(0);
}
function $floatHash(f) { return $floatToIntBits(f); }
function $doubleToString(d) { return $dstr(d); }
function $floatToString(f) { return $dstr(Math.fround(f)); }
function $charCode(c) { return c.charCodeAt(0); }
// The tables of tests/support/CharTables.java, from JDK 24's java.lang.Character: the zero of
// every run of ten decimal digits among the UTF-16 units, and every run of the other numeric
// values as its first unit, its length, the first one's value and 1 where the value grows by one
// along the run (a fraction's value is -2). Data of the Unicode Character Database 16.0.0, under
// the Unicode License V3, which NOTICE carries.
const $digitZeros = "\u0030\u0660\u06f0\u07c0\u0966\u09e6\u0a66\u0ae6\u0b66\u0be6\u0c66\u0ce6\u0d66\u0de6\u0e50\u0ed0\u0f20\u1040\u1090\u17e0\u1810\u1946\u19d0\u1a80\u1a90\u1b50\u1bb0\u1c40\u1c50\ua620\ua8d0\ua900\ua9d0\ua9f0\uaa50\uabf0\uff10";
const $numericRuns = [65, 26, 10, 1, 97, 26, 10, 1, 178, 2, 2, 1, 185, 1, 1, 0, 188, 3, -2, 0, 2548, 5, -2, 0, 2553, 1, 16, 0, 2930, 6, -2, 0, 3056, 1, 10, 0, 3057, 1, 100, 0, 3058, 1, 1000, 0, 3192, 4, 0, 1, 3196, 3, 1, 1, 3416, 7, -2, 0, 3440, 1, 10, 0, 3441, 1, 100, 0, 3442, 1, 1000, 0, 3443, 6, -2, 0, 3882, 10, -2, 0, 4969, 10, 1, 1, 4979, 1, 20, 0, 4980, 1, 30, 0, 4981, 1, 40, 0, 4982, 1, 50, 0, 4983, 1, 60, 0, 4984, 1, 70, 0, 4985, 1, 80, 0, 4986, 1, 90, 0, 4987, 1, 100, 0, 4988, 1, 10000, 0, 5870, 3, 17, 1, 6128, 10, 0, 1, 6618, 1, 1, 0, 8304, 1, 0, 0, 8308, 6, 4, 1, 8320, 10, 0, 1, 8528, 15, -2, 0, 8543, 2, 1, 0, 8545, 11, 2, 1, 8556, 1, 50, 0, 8557, 1, 100, 0, 8558, 1, 500, 0, 8559, 1, 1000, 0, 8560, 12, 1, 1, 8572, 1, 50, 0, 8573, 1, 100, 0, 8574, 1, 500, 0, 8575, 2, 1000, 0, 8577, 1, 5000, 0, 8578, 1, 10000, 0, 8581, 1, 6, 0, 8582, 1, 50, 0, 8583, 1, 50000, 0, 8584, 1, 100000, 0, 8585, 1, 0, 0, 9312, 20, 1, 1, 9332, 20, 1, 1, 9352, 20, 1, 1, 9450, 1, 0, 0, 9451, 10, 11, 1, 9461, 10, 1, 1, 9471, 1, 0, 0, 10102, 10, 1, 1, 10112, 10, 1, 1, 10122, 10, 1, 1, 11517, 1, -2, 0, 12295, 1, 0, 0, 12321, 9, 1, 1, 12344, 1, 10, 0, 12345, 1, 20, 0, 12346, 1, 30, 0, 12690, 4, 1, 1, 12832, 10, 1, 1, 12872, 1, 10, 0, 12873, 1, 20, 0, 12874, 1, 30, 0, 12875, 1, 40, 0, 12876, 1, 50, 0, 12877, 1, 60, 0, 12878, 1, 70, 0, 12879, 1, 80, 0, 12881, 15, 21, 1, 12928, 10, 1, 1, 12977, 15, 36, 1, 42726, 9, 1, 1, 42735, 1, 0, 0, 43056, 6, -2, 0, 63851, 1, 3, 0, 63859, 1, 10, 0, 63864, 1, 2, 0, 63922, 1, 0, 0, 63953, 1, 6, 0, 63955, 1, 6, 0, 63997, 1, 10, 0, 65313, 26, 10, 1, 65345, 26, 10, 1];
// A decimal digit's value, -1 for anything else.
function $decimal(c) {
  const u = c.charCodeAt(0);
  for (let i = 0; i < $digitZeros.length; i++) {
    const d = u - $digitZeros.charCodeAt(i);
    if (d >= 0 && d < 10) return d;
  }
  return -1;
}
// Character.digit: a decimal digit, or a Latin letter, ASCII or fullwidth, as 10 to 35.
function $digit(c, radix) {
  if (!(radix >= 2 && radix <= 36)) return -1;
  const u = c.charCodeAt(0);
  const lower = (u >= 0xff21 ? u - 0xfee0 : u) | 0x20;
  const d = lower >= 0x61 && lower <= 0x7a ? lower - 0x57 : $decimal(c);
  return d < radix ? d : -1;
}
function $numericValue(c) {
  const d = $decimal(c);
  if (d >= 0) return d;
  const u = c.charCodeAt(0);
  for (let i = 0; i < $numericRuns.length; i += 4) {
    const k = u - $numericRuns[i];
    if (k >= 0 && k < $numericRuns[i + 1]) return $numericRuns[i + 2] + k * $numericRuns[i + 3];
  }
  return -1;
}
function $roundToLong(d) { return $d2l(Math.floor(d + 0.5)); }
function $floorDivInt(a, b) { return $floorDiv(a, b); }
function $floorModInt(a, b) { return $floorMod(a, b); }
function $floorDivLong(a, b) { return $floorDiv(a, b); }
function $floorModLong(a, b) { return $floorMod(a, b); }
function $addExact(a, b) {
  const r = a + b;
  if (r > 2147483647 || r < -2147483648) $fail("ArithmeticException", "integer overflow");
  return r;
}
function $multiplyExact(a, b) {
  const r = a * b;
  if (r > 2147483647 || r < -2147483648) $fail("ArithmeticException", "integer overflow");
  return r;
}
function $subtractExact(a, b) {
  const r = a - b;
  if (r > 2147483647 || r < -2147483648) $fail("ArithmeticException", "integer overflow");
  return r;
}
function $longExact(r) {
  if (r !== BigInt.asIntN(64, r)) $fail("ArithmeticException", "long overflow");
  return r;
}
function $toIntExact(l) {
  if (l > 2147483647n || l < -2147483648n) $fail("ArithmeticException", "integer overflow");
  return Number(l);
}
function $arraycopy(src, srcPos, dest, destPos, length) {
  if (src === null || dest === null) $fail("NullPointerException", "arraycopy");
  if (srcPos < 0 || destPos < 0 || length < 0 || srcPos + length > src.length || destPos + length > dest.length) {
    $fail("ArrayIndexOutOfBoundsException", "arraycopy: last source index " + (srcPos + length) + " out of bounds for length " + src.length);
  }
  if (src === dest && srcPos < destPos) {
    for (let i = length - 1; i >= 0; i--) dest[destPos + i] = src[srcPos + i];
  } else {
    for (let i = 0; i < length; i++) dest[destPos + i] = src[srcPos + i];
  }
}
function $requireNonNull(x) {
  if (x === null) $fail("NullPointerException", "");
  return x;
}
function $arrayCopyOf(a, n) {
  const out = a.slice(0, n);
  for (let i = a.length; i < n; i++) out.push(null);
  return out;
}
function $arrayCopyOfRange(a, from, to) {
  if (from > to) $fail("IllegalArgumentException", from + " > " + to);
  if (from < 0 || from > a.length) $fail("ArrayIndexOutOfBoundsException", "Array index out of range: " + from);
  const out = a.slice(from, to);
  for (let i = out.length; i < to - from; i++) out.push(null);
  return out;
}
function $sortArray(a, comparator) {
  const cmp = comparator === null
    ? (x, y) => (typeof x === "string" ? $compareStrings(x, y) : typeof x === "number" ? $compareDoubles(x, y) : x < y ? -1 : x > y ? 1 : 0)
    : (x, y) => comparator.compare(x, y);
  a.sort(cmp);
}
function $sortRange(a, from, to, comparator) {
  const part = a.slice(from, to);
  $sortArray(part, comparator);
  for (let i = 0; i < part.length; i++) a[from + i] = part[i];
}
function $arraysEqual(a, b) {
  if (a === b) return true;
  if (a === null || b === null || a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (!$eq(a[i], b[i])) return false;
  return true;
}
function $arraysHash(a) {
  if (a === null) return 0;
  let h = 1;
  for (let i = 0; i < a.length; i++) h = (Math.imul(31, h) + (a[i] === null ? 0 : $hashCode(a[i]))) | 0;
  return h;
}
function $arraysToString(a) {
  if (a === null) return "null";
  let s = "[";
  for (let i = 0; i < a.length; i++) {
    if (i > 0) s += ", ";
    s += $str(a[i]);
  }
  return s + "]";
}
function $compareDoubles(a, b) {
  if (a < b) return -1;
  if (a > b) return 1;
  if (a === b) return a !== 0 ? 0 : (Object.is(a, b) ? 0 : Object.is(a, -0) ? -1 : 1);
  return a !== a ? (b !== b ? 0 : 1) : -1;
}
// Java's replace of a literal: `$&` and `$1` in the replacement are text, not a pattern.
function $replace(s, target, replacement) {
  return s.replaceAll(target, replacement.indexOf("$") < 0 ? replacement : () => replacement);
}
function $repeat(s, n) {
  if (n < 0) $fail("IllegalArgumentException", "count is negative: " + n);
  return s.repeat(n);
}
// Java's code point as a string; Scala.js throws where the JVM's `indexOf` answers -1.
function $fromCodePoint(c) {
  if (c >>> 0 > 0x10ffff) $fail("IllegalArgumentException", null);
  return String.fromCodePoint(c);
}
function $substring(s, a, b = s.length) {
  if (a < 0 || b > s.length || a > b) $fail("StringIndexOutOfBoundsException", "Range [" + a + ", " + b + ") out of bounds for length " + s.length);
  return s.substring(a, b);
}
function $codePointAt(s, i) {
  if (i < 0 || i >= s.length) $fail("StringIndexOutOfBoundsException", "Index " + i + " out of bounds for length " + s.length);
  return s.codePointAt(i);
}
function $charAt(s, i) {
  if (i < 0 || i >= s.length) $fail("StringIndexOutOfBoundsException", "Index " + i + " out of bounds for length " + s.length);
  return s[i];
}
// String.split of Java: the separator is a regular expression, a leading empty string stays and
// trailing ones go. A single character is always taken literally, as `split(Char)` needs.
function $split(s, sep, limit) {
  if (limit === undefined) limit = 0;
  if (sep.length > 1 && /[\\^$.|?*+()[\]{}]/.test(sep)) return $reSplit(sep, s, limit);
  // A surrogate is split at as a character, so never inside a pair.
  if (sep.length === 0 || /[\ud800-\udfff]/.test(sep)) return $reSplit(sep, s, limit);
  if (s.indexOf(sep) < 0) return [s];
  let parts = s.split(sep);
  if (limit > 0 && parts.length > limit) parts = parts.slice(0, limit - 1).concat(parts.slice(limit - 1).join(sep));
  if (limit === 0) while (parts.length > 0 && parts[parts.length - 1] === "") parts.pop();
  return parts;
}
function $isWhitespace(c) { return /[\t-\r\x1c-\x1f \u1680\u2000-\u2006\u2008-\u200a\u2028\u2029\u205f\u3000]/.test(c); }
// Java's trim takes off every character up to U+0020, strip the white space of isWhitespace.
function $trim(s) {
  let a = 0, b = s.length;
  if (b === 0 || (s.charCodeAt(0) > 32 && s.charCodeAt(b - 1) > 32)) return s;
  while (a < b && s.charCodeAt(a) <= 32) a++;
  while (b > a && s.charCodeAt(b - 1) <= 32) b--;
  return s.slice(a, b);
}
function $strip(s, lead, trail) {
  let a = 0, b = s.length;
  if (lead) while (a < b && $isWhitespace(s[a])) a++;
  if (trail) while (b > a && $isWhitespace(s[b - 1])) b--;
  return s.slice(a, b);
}
function $stripMargin(s, margin) {
  return s.split("\n").map((line) => {
    const i = line.indexOf(margin);
    return i >= 0 && line.slice(0, i).trim() === "" ? line.slice(i + 1) : line;
  }).join("\n");
}
const $escapes = { n: "\n", t: "\t", r: "\r", b: "\b", f: "\f", "\\": "\\", "\"": "\"", "'": "'" };
function $processEscapes(s) {
  if (s.indexOf("\\") < 0) return s;
  return s.replace(/\\(u+[0-9a-fA-F]{4}|[^])/g, (m, e) => {
    if (e.length > 1) return String.fromCharCode(parseInt(e.slice(-4), 16));
    const c = $escapes[e];
    if (c === undefined) $fail("IllegalArgumentException", "invalid escape '" + m + "' in \"" + s + "\"");
    return c;
  });
}
// Keys compared structurally: objects, and the functions that instances of a class with a
// function type among its parents are.
function $structural(k) {
  return k !== null && (typeof k === "object" || (typeof k === "function" && k.$className !== undefined));
}
// Insertion-ordered hash map with structural key equality.
class $HMap {
  constructor() { this.m = new Map(); this.b = null; }
  canon(k, insert) {
    if (!$structural(k)) return k;
    if (this.b === null) {
      if (!insert) return undefined;
      this.b = new Map();
    }
    const h = $hash(k);
    let bucket = this.b.get(h);
    if (bucket !== undefined) {
      for (let i = 0; i < bucket.length; i++) if ($eq(bucket[i], k)) return bucket[i];
    }
    if (!insert) return undefined;
    if (bucket === undefined) this.b.set(h, [k]); else bucket.push(k);
    return k;
  }
  has(k) { const c = this.canon(k, false); return c !== undefined && this.m.has(c); }
  get(k) { const c = this.canon(k, false); return c === undefined ? undefined : this.m.get(c); }
  set(k, v) { this.m.set(this.canon(k, true), v); }
  delete(k) {
    const c = this.canon(k, false);
    if (c === undefined || !this.m.delete(c)) return false;
    if ($structural(c)) {
      const h = $hash(c);
      const bucket = this.b.get(h).filter((x) => x !== c);
      if (bucket.length === 0) this.b.delete(h); else this.b.set(h, bucket);
    }
    return true;
  }
  clone() {
    const r = new $HMap();
    r.m = new Map(this.m);
    if (this.b !== null) {
      r.b = new Map();
      for (const [h, bucket] of this.b) r.b.set(h, bucket.slice());
    }
    return r;
  }
  keys() { return Array.from(this.m.keys()); }
  values() { return Array.from(this.m.values()); }
  size() { return this.m.size; }
  forEach(f) {
    const ks = this.keys(), vs = this.values();
    for (let i = 0; i < ks.length; i++) f(ks[i], vs[i]);
  }
  clear() { this.m.clear(); this.b = null; }
}
// The key equality of the immutable Map and Set (`keyEquals` in std/maps.scala): what a native
// Map gives for numbers, strings and the rest, and `equals` between two objects.
function $keyEq(a, b) {
  if (a === b) return true;
  if (a !== a) return b !== b;
  return $structural(a) && $structural(b) && $eq(a, b);
}
// A dictionary as the entries of a mutable map with string keys (`js.Any.wrapDictionary`): the
// map reads and writes the dictionary's own enumerable properties.
class $DictMap {
  constructor(d) { this.d = d; }
  get m() { return { size: this.size() }; }
  has(k) { return Object.prototype.hasOwnProperty.call(this.d, k); }
  get(k) { return this.has(k) ? this.d[k] : undefined; }
  set(k, v) { this.d[k] = v; }
  delete(k) {
    if (!this.has(k)) return false;
    delete this.d[k];
    return true;
  }
  keys() { return Object.keys(this.d); }
  values() { return Object.keys(this.d).map((k) => this.d[k]); }
  size() { return Object.keys(this.d).length; }
  forEach(f) { for (const k of Object.keys(this.d)) f(k, this.d[k]); }
  clear() { for (const k of Object.keys(this.d)) delete this.d[k]; }
}
function $mapEquals(a, b) {
  if (a.size() !== b.size()) return false;
  const ks = a.keys(), vs = a.values();
  for (let i = 0; i < ks.length; i++) if (!b.has(ks[i]) || !$eq(vs[i], b.get(ks[i]))) return false;
  return true;
}
// MurmurHash3.mapHash: the unordered hash of the entries as Tuple2s.
function $mapHash(m) {
  let a = 0, b = 0, c = 1, n = 0;
  m.forEach((k, v) => {
    const e = $finalizeHash($mix($mix($mix($productSeed, $tuple2Prefix), $hash(k)), $hash(v)), 2);
    a = (a + e) | 0;
    b ^= e;
    c = Math.imul(c, e | 1);
    n++;
  });
  return $unorderedHash(a, b, c, n, $mapSeed);
}
function $jsObj(...pairs) {
  const o = {};
  for (const p of pairs) o[p._1] = p._2;
  return o;
}
function $jsSet(target, key, value) { target[key] = value; }
// ---- regular expressions with the semantics of java.util.regex ----
const $reCache = new Map();
const $reSpace = " \\t\\n\\x0B\\f\\r";
const $reHSpace = " \\t\\xA0\\u1680\\u180e\\u2000-\\u200a\\u202f\\u205f\\u3000";
const $reVSpace = "\\n\\x0B\\f\\r\\x85\\u2028\\u2029";
// The complements of the three sets for a class, which cannot hold a negated one.
const $reNotSpace = "\\0-\\x08\\x0E-\\x1F\\x21-\\u{10FFFF}";
const $reNotHSpace = "\\0-\\x08\\x0A-\\x1F\\x21-\\x9F\\xA1-\\u167F\\u1681-\\u180D\\u180F-\\u1FFF\\u200B-\\u202E\\u2030-\\u205E\\u2060-\\u2FFF\\u3001-\\u{10FFFF}";
const $reNotVSpace = "\\0-\\x09\\x0E-\\x84\\x86-\\u2027\\u202A-\\u{10FFFF}";
const $reEnd = "(?=(?:\\r\\n|[\\n\\r\\u0085\\u2028\\u2029])?(?![\\s\\S]))";
// Under Java's `(?i)` without `(?u)` the ASCII letters alone match their other case, which
// JavaScript's `i` would give every letter: a letter becomes the class of its two cases.
function $reOtherCase(cp) {
  return cp >= 65 && cp <= 90 ? cp + 32 : cp >= 97 && cp <= 122 ? cp - 32 : -1;
}
// `\Q...\E` written out as the characters it quotes, as the JDK's `Pattern.RemoveQEQuoting` does
// before it parses: inside a block an ASCII character that is neither a letter nor a digit gets a
// backslash, a digit that opens the block is `\x3N`, so that it cannot extend a back reference
// before it, and an empty block leaves nothing.
function $reUnquote(src) {
  let i = 0;
  while (i < src.length - 1) {
    if (src[i] !== "\\") i++;
    else if (src[i + 1] !== "Q") i += 2;
    else break;
  }
  if (i >= src.length - 1) return src;
  let out = src.slice(0, i);
  let inQuote = true, beginQuote = true;
  i += 2;
  while (i < src.length) {
    const c = src[i++];
    if (c.charCodeAt(0) > 127 || /[A-Za-z]/.test(c)) out += c;
    else if (c >= "0" && c <= "9") out += (beginQuote ? "\\x3" : "") + c;
    else if (c !== "\\") out += (inQuote ? "\\" : "") + c;
    else if (inQuote) {
      if (src[i] === "E") {
        i++;
        inQuote = false;
      } else out += "\\\\";
    } else if (src[i] === "Q") {
      i++;
      inQuote = beginQuote = true;
      continue;
    } else {
      out += c;
      if (i < src.length) out += src[i++];
    }
    beginQuote = false;
  }
  return out;
}
// Java's syntax in JavaScript's Unicode mode, read as the JDK's `Pattern` reads it: with its
// cursor, which in comments mode (`x`, lexical: a group restores it) passes over ASCII white
// space and `#` comments inside classes too, where the JDK's reads do and not where they take
// the next character as it stands (after a backslash, `(?`, `{`, and a class's `^` and range
// hyphen). A class is a set's expression, as `Pattern.clazz` and `range` parse it (nested
// classes, `&&`, the characters below U+0100 in one set that the class's operands share),
// written with lookaheads where a union, an intersection or a complement is no single class of
// the engine's, so that every pattern stays in Unicode mode. `(?i)` alone folds the ASCII letters
// by hand, each a class of its two cases, `(?iu)` takes the engine's folding, and under both
// `Lu`, `Ll` and `Lt` are the three categories together, as for the JDK; under the engine's
// folding a complemented property is a complemented class, which folds before it complements.
// Whether a pattern compiles: null, or the message of what it fails at.
function $reCheck(src) {
  try {
    const [body, flags] = $reTranslate(src);
    new RegExp(body, flags);
    return null;
  } catch (e) { return e.message; }
}
// The flag groups at the head of a pattern become the engine's flags, and the result is the
// pattern and its flags.
function $reTranslate(source) {
  const src = $reUnquote(source);
  let at = 0, x = false, ci = false, uc = false, multiline = false, dotall = false, groups = 0, out = "";
  const scopes = [];
  const fail = (message) => { throw new SyntaxError(message); };
  const past = () => {
    while (x) {
      const c = src[at];
      if (c === "#") while (++at < src.length && !"\n\r\x85\u2028\u2029\0".includes(src[at]));
      else if (c !== undefined && " \t\n\x0B\f\r".includes(c)) at++;
      else break;
    }
  };
  const peek = () => (past(), src[at]);
  const next = () => {
    at += src.codePointAt(at) > 0xffff ? 2 : 1;
    return peek();
  };
  const read = () => {
    const c = peek() === undefined ? undefined : String.fromCodePoint(src.codePointAt(at));
    if (c !== undefined) at += c.length;
    return c;
  };
  const hex = (c) => c !== undefined && /^[0-9a-fA-F]$/.test(c);
  const digit = (c) => c >= "0" && c <= "9";
  const lit = (cp, inClass) => {
    const c = String.fromCodePoint(cp);
    return "^$\\.*+?()[]{}|/".includes(c) || (inClass && c === "-") ? "\\" + c : cp < 32 || cp > 126 || (!inClass && digit(c)) ? "\\u{" + cp.toString(16) + "}" : c;
  };
  const fold = (cp) => (ci && !uc ? $reOtherCase(cp) : -1);
  const name = () => {
    let s = "";
    for (let c; (c = read()) !== ">"; s += c) if (c === undefined) fail("named capturing group is missing trailing '>'");
    return s;
  };
  // The escape at `at`: a character ({ cp }), a predefined class ({ set } its node in a class,
  // `out` its text outside one), or what it stands for outside a class ({ out }).
  const escape = (inClass, isRange) => {
    const cp = src.codePointAt(++at);
    if (cp === undefined) fail("Unexpected internal error");
    const n = String.fromCodePoint(cp);
    at += n.length;
    const named = "tnrfae".indexOf(n);
    if (named >= 0) return { cp: [9, 10, 13, 12, 7, 27][named] };
    if (n === "0") {
      const a = read(), b = read(), c = b >= "0" && b <= "7" ? read() : b;
      if (!(a >= "0" && a <= "7")) fail("Illegal octal escape sequence");
      if (!(b >= "0" && b <= "7")) return at -= b ? b.length : 0, { cp: +a };
      if (c >= "0" && c <= "7" && a <= "3") return { cp: a * 64 + b * 8 + +c };
      return at -= c ? c.length : 0, { cp: a * 8 + +b };
    }
    const hexes = (count, message) => {
      let v = 0;
      for (let k = 0; k < count; k++) {
        const c = read();
        if (!hex(c)) fail(message);
        v = v * 16 + parseInt(c, 16);
      }
      return v;
    };
    if (n === "x") {
      if (peek() !== "{") return { cp: hexes(2, "Illegal hexadecimal escape sequence") };
      at++;
      if (!hex(peek())) fail("Illegal hexadecimal escape sequence");
      let v = 0, c;
      while (hex((c = read()))) if ((v = v * 16 + parseInt(c, 16)) > 0x10ffff) fail("Hexadecimal codepoint is too big");
      if (c !== "}") fail("Unclosed hexadecimal escape sequence");
      return { cp: v };
    }
    if (n === "u") {
      const v = hexes(4, "Illegal Unicode escape sequence"), mark = at;
      if (v >= 0xd800 && v <= 0xdbff && read() === "\\" && read() === "u") {
        const w = hexes(4, "Illegal Unicode escape sequence");
        if (w >= 0xdc00 && w <= 0xdfff) return { cp: 0x10000 + ((v - 0xd800) << 10) + (w - 0xdc00) };
      }
      at = mark;
      return { cp: v };
    }
    if (n === "c") {
      const c = at < src.length ? read() : undefined;
      if (c === undefined) fail("Illegal control escape sequence");
      return { cp: c.codePointAt(0) ^ 64 };
    }
    const k = "dDwWsShHvV".indexOf(n);
    if (k >= 4 && !(n === "v" && isRange)) {
      const plain = [$reSpace, $reHSpace, $reVSpace][(k - 4) >> 1];
      return k % 2 ? { set: { k: "[", t: [$reNotSpace, $reNotHSpace, $reNotVSpace][(k - 4) >> 1] }, out: "[^" + plain + "]" } : { set: { k: "[", t: plain }, out: "[" + plain + "]" };
    }
    if (k >= 0) return k < 4 ? { set: { k: "[", t: "\\" + n }, out: "\\" + n } : { cp: 11 };
    if (!inClass) {
      if (n >= "1" && n <= "9") {
        // Java takes a further digit into a back reference while that group exists so far.
        let g = +n;
        while (digit(peek()) && g * 10 + +src[at] <= groups) g = g * 10 + +src[at++];
        return { out: digit(peek()) ? "(?:\\" + g + ")" : "\\" + g };
      }
      if (n === "k") {
        if (read() !== "<") fail("\\k is not followed by '<' for named capturing group");
        return { out: "\\k<" + name() + ">" };
      }
      const anchor = { A: "(?<![\\s\\S])", z: "(?![\\s\\S])", Z: $reEnd, R: "(?:\\r\\n|[" + $reVSpace + "])" }[n];
      if (anchor !== undefined) return { out: anchor };
      if ("bBGXN".includes(n)) return { out: "\\" + n };
    }
    if (/[0-9A-Za-z]/.test(n)) fail("Illegal/unsupported escape sequence");
    return { cp };
  };
  // `\p` or `\P` with `at` at its letter.
  const family = (neg) => {
    let p;
    if (next() === "{") {
      next();
      const i = at;
      for (let c; (c = read()) !== "}"; ) if (c === undefined) fail("Unclosed character family");
      if (!(p = src.slice(i, at - 1))) fail("Empty character family");
    } else if ((p = read()) === undefined) fail("Unknown character property");
    // Java's POSIX classes the engine would take as Unicode properties are ASCII ones, of either
    // case under `(?i)`; the engine refuses the others.
    if (/^(?:Lower|Upper|Alpha)$/.test(p)) return { k: "[", t: ci || p === "Alpha" ? "a-zA-Z" : p === "Lower" ? "a-z" : "A-Z", neg };
    if (ci && /^L[ult]$/.test(p.replace(/^(?:gc|general_category)=/i, ""))) p = "LC";
    const t = "\\p{" + p + "}";
    return neg && ci && uc ? { k: "[", t, neg } : { k: "[", t: neg ? "\\P" + t.slice(2) : t };
  };
  // A character of a class below U+0100 joins the class's one set, which its operands share as
  // it is at the class's end; under `(?iu)` those whose other case lies above it do not.
  const bitsOrSingle = (bits, cp) => {
    if (cp < 256 && !(ci && uc && [0xff, 0xb5, 0x49, 0x69, 0x53, 0x73, 0x4b, 0x6b, 0xc5, 0xe5].includes(cp))) {
      const o = fold(cp);
      bits.t += lit(cp, true) + (o < 0 ? "" : lit(o, true));
      return null;
    }
    return { k: "[", t: lit(cp, true) };
  };
  const range = (bits) => {
    let cp;
    if (peek() === "\\") {
      const n = src[at + 1];
      if (n === "p" || n === "P") return at++, family(n === "P");
      const e = escape(true, src[at + 2] === "-");
      if (e.set !== undefined) return e.set;
      cp = e.cp;
    } else {
      cp = src.codePointAt(at);
      next();
    }
    if (peek() === "-" && src[at + 1] !== "[" && src[at + 1] !== "]") {
      next();
      let hi;
      if (peek() === "\\") hi = escape(true, true).cp ?? -1;
      else {
        hi = src.codePointAt(at) ?? -1;
        next();
      }
      if (hi < cp) fail("Illegal character range");
      let t = lit(cp, true) + "-" + lit(hi, true);
      for (const [a, b] of ci && !uc ? [[97, 122], [65, 90]] : []) {
        const lo = Math.max(cp, a), up = Math.min(hi, b);
        if (lo <= up) t += lit($reOtherCase(lo), true) + "-" + lit($reOtherCase(up), true);
      }
      return { k: "[", t };
    }
    return bitsOrSingle(bits, cp);
  };
  const or = (a, b) => (a === null ? b : { k: "|", a, b });
  // Whether the union `a` holds `b` among its members.
  const holds = (a, b) => a === b || (a.k === "|" && (holds(a.a, b) || holds(a.b, b)));
  // An empty operand intersects what precedes with its last element, which the one holds or the
  // other is an intersection with: the smaller of the two, so that neither is written twice.
  const and = (a, b) => (holds(a, b) ? b : a.k === "&" && a.b === b ? a : { k: "&", a, b });
  // A class from its `[` (`consume` then takes its `]`), or the operand of an `&&` that no
  // brackets delimit.
  const clazz = (consume) => {
    const bits = { k: "[", t: "" };
    let prev = null, curr = null, hasBits = false, c = next(), neg = c === "^" && src[at - 1] === "[";
    if (neg) c = next();
    for (;;) {
      if (c === "[") {
        prev = or(prev, (curr = clazz(true)));
        c = peek();
        continue;
      }
      if (c === "&") {
        if (next() === "&") {
          let right = null;
          for (c = next(); c !== "]" && c !== "&"; c = peek()) {
            if (c !== "[") at--;
            right = or(right, clazz(c === "["));
          }
          if (hasBits) {
            prev = prev === null ? (curr = bits) : or(prev, bits);
            hasBits = false;
          }
          if (right !== null) curr = right;
          if (prev === null) prev = right ?? fail("Bad class syntax");
          else prev = and(prev, curr ?? fail("Bad intersection syntax"));
          continue;
        }
        at--;
      } else if (c === undefined) fail("Unclosed character class");
      else if (c === "]" && (prev !== null || hasBits)) {
        if (consume) next();
        prev = prev === null ? bits : hasBits ? or(prev, bits) : prev;
        return neg ? { k: "^", a: prev } : prev;
      }
      curr = range(bits);
      if (curr === null) hasBits = true;
      else prev = or(prev, curr);
      c = peek();
    }
  };
  const body = (n) => {
    if (n.k === "[") return n.neg ? null : n.t;
    const a = n.k === "|" ? body(n.a) : null, b = a === null ? null : body(n.b);
    return b === null ? null : a + b;
  };
  const emit = (n) => {
    const b = body(n);
    if (b !== null) return "[" + b + "]";
    if (n.k === "[") return "[^" + n.t + "]";
    if (n.k === "|") return "(?:" + emit(n.a) + "|" + emit(n.b) + ")";
    if (n.k === "&") return "(?:(?=" + emit(n.a) + ")" + emit(n.b) + ")";
    const inner = body(n.a);
    return inner !== null ? "[^" + inner + "]" : n.a.k === "[" ? "[" + n.a.t + "]" : "(?:(?!" + emit(n.a) + ")[\\s\\S])";
  };
  const group = () => {
    const outer = x;
    at++;
    if (peek() !== "?") {
      groups++;
      out += "(";
    } else if (src[++at] === "<") {
      at++;
      const c = read();
      if (c === "=" || c === "!") out += "(?<" + c;
      else {
        at -= c ? c.length : 0;
        groups++;
        out += "(?<" + name() + ">";
      }
    } else if (src[at] !== undefined && ":=!>".includes(src[at])) out += "(?" + src[at++];
    else {
      let on = true, flags = "";
      for (let c = peek(); ; c = next()) {
        if (c === "-" && on) on = false, flags += c;
        else if (c === "x") x = on;
        else if (c !== undefined && "imsducU".includes(c)) flags += c;
        else break;
      }
      flags = flags.replace(/-$/, "");
      const end = read();
      if (end === ")") {
        if (out === "" && !scopes.length && /^[imsu]*$/.test(flags)) {
          ci ||= flags.includes("i");
          uc ||= flags.includes("u");
          multiline ||= flags.includes("m");
          dotall ||= flags.includes("s");
        } else if (flags) out += "(?" + flags + ")";
        return;
      }
      if (end !== ":") fail("Unknown inline modifier");
      out += "(?" + flags + ":";
    }
    scopes.push(outer);
  };
  for (let c; (c = peek()) !== undefined; ) {
    if (c === "\\") {
      const n = src[at + 1];
      if (n === "p" || n === "P") {
        at++;
        out += emit(family(n === "P"));
      } else {
        const e = escape(false);
        const o = e.cp === undefined ? -1 : fold(e.cp);
        out += e.cp === undefined ? e.out : o < 0 ? lit(e.cp) : "[" + lit(e.cp, true) + lit(o, true) + "]";
      }
    } else if (c === "[") out += emit(clazz(true));
    else if (c === "(") group();
    else if (c === ")") {
      at++;
      if (scopes.length) x = scopes.pop();
      out += c;
    } else if (c === "{") {
      if (!digit(src[++at])) fail("Illegal repetition");
      let q = "{" + src[at++], d;
      while (digit((d = read()))) q += d;
      if (d === ",") for (q += d; digit((d = read())); ) q += d;
      const [lo, hi] = q.slice(1).split(",");
      if (d !== "}") fail("Unclosed counted closure");
      if (+lo > 2147483647 || +hi > 2147483647 || (hi && +hi < +lo)) fail("Illegal repetition range");
      out += q + d;
    } else {
      const cp = src.codePointAt(at), t = String.fromCodePoint(cp), o = fold(cp);
      at += t.length;
      out += c === "$" ? (multiline ? c : $reEnd) : c === "]" || c === "}" ? "\\" + c : o < 0 ? t : "[" + t + String.fromCharCode(o) + "]";
    }
  }
  return [out, (multiline ? "m" : "") + (dotall ? "s" : "") + (ci && uc ? "i" : "") + "u"];
}
// mode: "" first match, "g" all matches, "y" match at the start, "f" match of the whole input;
// "d" adds the groups' positions.
function $re(src, mode) {
  const key = mode + src;
  let re = $reCache.get(key);
  if (re === undefined) {
    try {
      const [body, flags] = $reTranslate(src);
      re = new RegExp(mode.includes("f") ? "(?<![\\s\\S])(?:" + body + ")(?![\\s\\S])" : body, mode.replace("f", "") + flags);
    } catch (e) { $fail("IllegalArgumentException", "PatternSyntaxException: " + e.message); }
    if ($reCache.size > 2000) $reCache.clear();
    $reCache.set(key, re);
  }
  re.lastIndex = 0;
  return re;
}
function $reExec(src, mode, s) { return $re(src, mode).exec(s); }
function $reAll(src, s, mode = "g") { return Array.from(s.matchAll($re(src, mode))); }
// Replacement strings of Matcher.appendReplacement: $n, ${name} and backslash escapes.
function $reExpand(repl, m) {
  if (repl.indexOf("$") < 0 && repl.indexOf("\\") < 0) return repl;
  let out = "";
  for (let i = 0; i < repl.length; i++) {
    const c = repl[i];
    if (c === "\\") {
      i++;
      if (i >= repl.length) $fail("IllegalArgumentException", "character to be escaped is missing");
      out += repl[i];
    } else if (c === "$") {
      i++;
      if (repl[i] === "{") {
        const end = repl.indexOf("}", i);
        const name = repl.slice(i + 1, end < 0 ? repl.length : end);
        if (end < 0 || m.groups === undefined || !(name in m.groups)) $fail("IllegalArgumentException", "No group with name {" + name + "}");
        out += m.groups[name] ?? "";
        i = end;
      } else {
        let n = repl.charCodeAt(i) - 48;
        if (!(n >= 0 && n <= 9)) $fail("IllegalArgumentException", "Illegal group reference");
        if (n > m.length - 1) $fail("IndexOutOfBoundsException", "No group " + n);
        while (i + 1 < repl.length) {
          const d = repl.charCodeAt(i + 1) - 48;
          if (!(d >= 0 && d <= 9) || n * 10 + d > m.length - 1) break;
          n = n * 10 + d;
          i++;
        }
        out += m[n] ?? "";
      }
    } else out += c;
  }
  return out;
}
// repl is a replacement string or a function from the wrapped match to one.
function $reReplace(src, s, repl, all, wrap) {
  let out = "";
  let last = 0;
  const step = (m) => {
    out += s.slice(last, m.index) + $reExpand(typeof repl === "function" ? repl(wrap(m)) : repl, m);
    last = m.index + m[0].length;
  };
  if (all) for (const m of s.matchAll($re(src, "g"))) step(m);
  else {
    const m = $re(src, "").exec(s);
    if (m !== null) step(m);
  }
  return out + s.slice(last);
}
function $reSplit(src, s, limit) {
  const re = $re(src, "g");
  if (s.length === 0) return [""];
  const parts = [];
  let last = 0;
  for (const m of s.matchAll(re)) {
    if (limit > 0 && parts.length === limit - 1) break;
    const end = m.index + m[0].length;
    if (end === 0) continue;
    parts.push(s.slice(last, m.index));
    last = end;
  }
  if (parts.length === 0) return [s];
  parts.push(s.slice(last));
  if (limit === 0) while (parts.length > 0 && parts[parts.length - 1] === "") parts.pop();
  return parts;
}
// ---- java.util.Formatter: the f interpolator and String.format ----
// A decimal literal as sign, digits and the exponent of 0.DIGITS.
function $decParts(x) {
  const m = /^([+-]?)(\d*)\.?(\d*)(?:[eE]([+-]?\d+))?$/.exec(typeof x === "string" ? x : String(x));
  if (m === null) throw new Error("IllegalFormatConversionException: " + $str(x));
  let digits = m[2] + m[3];
  let point = m[2].length + (m[4] === undefined ? 0 : parseInt(m[4], 10));
  const lead = /^0*/.exec(digits)[0].length;
  digits = digits.slice(lead);
  point -= lead;
  if (digits === "") return { neg: m[1] === "-" || Object.is(x, -0), digits: "0", point: 1, zero: true };
  return { neg: m[1] === "-", digits, point, zero: false };
}
// The first `count` digits, rounded half up as Java does on the decimal expansion.
function $roundDigits(digits, count) {
  if (count >= digits.length) return BigInt(digits + "0".repeat(count - digits.length));
  if (count < 0) return 0n;
  const kept = BigInt(digits.slice(0, count) || "0");
  return digits.charCodeAt(count) >= 53 ? kept + 1n : kept;
}
function $fmtFixed(p, precision, grouping) {
  let text = $roundDigits(p.digits, p.point + precision).toString().padStart(precision + 1, "0");
  let int = precision > 0 ? text.slice(0, -precision) : text;
  if (grouping) int = int.replace(/\B(?=(\d{3})+$)/g, ",");
  return precision > 0 ? int + "." + text.slice(-precision) : int;
}
function $fmtExp(p, precision) {
  let exp = p.zero ? 0 : p.point - 1;
  let text = $roundDigits(p.digits, precision + 1).toString();
  if (text.length > precision + 1) {
    text = text.slice(0, -1);
    exp++;
  }
  const mantissa = precision > 0 ? text[0] + "." + text.slice(1) : text;
  return mantissa + "e" + (exp < 0 ? "-" : "+") + String(Math.abs(exp)).padStart(2, "0");
}
function $fmtOne(flags, width, precision, conv, arg) {
  const has = (f) => flags.indexOf(f) >= 0;
  let sign = "";
  let body;
  let numeric = false;
  const lower = conv.toLowerCase();
  if (lower === "d" || lower === "x" || lower === "o") {
    if (typeof arg !== "bigint" && !(typeof arg === "number" && Number.isInteger(arg))) {
      if (arg != null && typeof arg === "object" && lower === "d") arg = BigInt(arg.toString());
      else throw new Error("IllegalFormatConversionException: " + conv + " != " + $str(arg));
    }
    numeric = true;
    if (lower === "d") {
      body = String(arg < 0 ? -arg : arg);
      if (arg < 0) sign = "-";
      if (has(",")) body = body.replace(/\B(?=(\d{3})+$)/g, ",");
    } else {
      const unsigned = typeof arg === "bigint" ? BigInt.asUintN(64, arg) : arg >>> 0;
      body = unsigned.toString(lower === "x" ? 16 : 8);
      if (has("#")) body = (lower === "x" ? "0x" : "0") + body;
    }
  } else if (lower === "f" || lower === "e") {
    if (typeof arg === "number" && !Number.isFinite(arg)) {
      body = arg !== arg ? "NaN" : "Infinity";
      if (arg < 0) sign = "-";
      flags = flags.replace("0", "");
    } else {
      const p = $decParts(typeof arg === "number" || typeof arg === "bigint" ? arg : arg.toString());
      numeric = true;
      if (p.neg) sign = "-";
      body = lower === "f" ? $fmtFixed(p, precision ?? 6, has(",")) : $fmtExp(p, precision ?? 6);
    }
  } else if (lower === "g") {
    if (typeof arg === "number" && !Number.isFinite(arg)) {
      body = arg !== arg ? "NaN" : "Infinity";
      if (arg < 0) sign = "-";
      flags = flags.replace("0", "");
    } else {
      const p = $decParts(typeof arg === "number" || typeof arg === "bigint" ? arg : arg.toString());
      numeric = true;
      if (p.neg) sign = "-";
      // `precision` significant digits: fixed between 1e-4 and 10^precision, scientific outside.
      const digits = precision === undefined ? 6 : Math.max(precision, 1);
      let exp = p.zero ? 0 : p.point - 1;
      if ($roundDigits(p.digits, digits).toString().length > digits) exp++;
      body = !p.zero && (exp < -4 || exp >= digits) ? $fmtExp(p, digits - 1) : $fmtFixed(p, digits - 1 - exp, has(","));
    }
  } else if (lower === "s") {
    body = $str(arg);
    if (precision !== undefined) body = body.slice(0, precision);
  } else if (lower === "c") body = typeof arg === "number" ? String.fromCodePoint(arg) : String(arg);
  else if (lower === "b") body = arg == null ? "false" : typeof arg === "boolean" ? String(arg) : "true";
  else throw new Error("UnknownFormatConversionException: Conversion = '" + conv + "'");
  if (numeric && sign === "") sign = has("+") ? "+" : has(" ") ? " " : "";
  if (numeric && sign === "-" && has("(")) {
    sign = "(";
    body += ")";
  }
  let text = sign + body;
  if (width !== undefined && text.length < width) {
    if (has("-")) text = text.padEnd(width, " ");
    else if (has("0") && numeric) text = sign + body.padStart(width - sign.length, "0");
    else text = text.padStart(width, " ");
  }
  return conv !== lower ? text.toUpperCase() : text;
}
const $fmtSpec = /%(?:(\d+)\$)?([-#+ 0,(]*)(\d+)?(?:\.(\d+))?([a-zA-Z%])/g;
function $format(fmt, args) {
  let next = 0;
  return fmt.replace($fmtSpec, (all, index, flags, width, precision, conv) => {
    if (conv === "%") return "%";
    if (conv === "n") return "\n";
    const i = index === undefined ? next++ : parseInt(index, 10) - 1;
    if (i >= args.length) throw new Error("MissingFormatArgumentException: Format specifier '" + all + "'");
    return $fmtOne(flags, width === undefined ? undefined : parseInt(width, 10),
      precision === undefined ? undefined : parseInt(precision, 10), conv, args[i]);
  });
}
// f"...": an argument is formatted by the specifier that follows it, or as %s without one.
function $formatInterpolated(parts, args) {
  let fmt = $processEscapes(parts[0]);
  for (let i = 1; i < parts.length; i++) {
    const part = $processEscapes(parts[i]);
    fmt += (/^%(?:[-#+ 0,(]*)(?:\d+)?(?:\.\d+)?[a-zA-Z]/.test(part) && !part.startsWith("%n") ? "" : "%s") + part;
  }
  return $format(fmt, args);
}
// --hot: a module that is not split per file takes the definitions of a per-file module through
// bindings of its own, which its setter assigns from what the module gives: when both have
// run, when the module's enum values exist, and again when a hot swap has run the module anew.
const $hotGiven = new Map();
const $hotUsers = new Map();
function $hotUse(module, set) {
  const users = $hotUsers.get(module);
  if (users === undefined) $hotUsers.set(module, [set]);
  else users.push(set);
  const given = $hotGiven.get(module);
  if (given !== undefined) set(given());
}
function $hotProvide(module, given = $hotGiven.get(module)) {
  $hotGiven.set(module, given);
  const users = $hotUsers.get(module);
  if (users !== undefined) {
    const names = given();
    for (const set of users) set(names);
  }
}
// --hot: what the page has run and what the last build holds, kept on the page itself, since a
// module run again may be this one. `ran` has the latest instance of every module by its name,
// with the hash of its text and the id of the build that wrote it; `build` and `buildId` the
// listing of the last `hot-build.mjs` (every module's hash and writer) and its build;
// `failed` the modules whose last update failed and `failedAt` the time of the last such
// update; `updating` the updates a server announced, by the path of the module that accepts
// each; `wish` why the page is to be loaded again; `left` the `$hot` of the modules a swap ran
// again. `pending()` answers what a wish waits for, for a check to read: nothing, or the wish
// with the listing's build, the modules whose text or writer the page has is not the one the
// build lists, and the failed ones. `failed_(module, at)` is for the module that accepts an
// update above main.mjs (sbt-teq's stub) to tell of one that failed, by the update's time or
// the module's address.
function $hotState() {
  const hot = globalThis.__teqHot ??= {
    ran: new Map(), build: undefined, buildId: undefined, builtAt: -1, failed: new Set(), failedAt: -1, updating: new Map(), wish: undefined, left: [], booted: false, reloading: false, api: undefined,
    pending() { return this.wish === undefined ? undefined : { wish: this.wish, listing: this.buildId, waits: $hotWaits(this), failed: [...this.failed] }; },
    failed_(module, at) { $hotFailed(this, module, at); },
  };
  if (hot.api !== import.meta.hot && import.meta.hot?.on !== undefined) {
    hot.api = import.meta.hot;
    import.meta.hot.on("vite:beforeUpdate", (payload) => { for (const u of payload.updates ?? []) hot.updating.set(u.acceptedPath.split("?")[0], u.timestamp); });
  }
  return hot;
}
// The name of the module at an address and the time of the update that loaded it there, which
// a development server puts into the address (`?t=`); none for a module of the first load.
function $hotAddress(url) {
  const file = url.slice(url.lastIndexOf("/") + 1);
  const query = file.indexOf("?");
  const name = query < 0 ? file : file.slice(0, query);
  const time = query < 0 ? null : /[?&]t=(\d+)/.exec(file.slice(query));
  return [name.endsWith(".mjs") ? name.slice(0, -4) : name, time === null ? 0 : Number(time[1])];
}
// The modules the page has run in another text, or from another build, than the listing has.
// A module the build does not list is gone from the build, one the page never ran is not the
// page's, and one whose update failed has no say.
function $hotWaits(hot) {
  const waits = [];
  if (hot.build !== undefined) {
    for (const [module, { hash, id }] of hot.ran) {
      const listed = hot.build[module];
      if (listed !== undefined && (listed[0] !== hash || listed[1] !== id) && !hot.failed.has(module)) waits.push({ module, has: hash + " " + id, listed: listed[0] + " " + listed[1] });
    }
  }
  return waits;
}
// A wish is carried out when the last listing has, for every module the page ran, the text
// the page ran and the build that wrote it: that build is published whole then, and its
// updates have arrived. Agreement of the texts alone would not do: a build that restores a
// text an earlier listing has could pass for that build while it is half published. A failed
// update, whose build the page cannot know, has the listing be of an update after it.
function $hotSettle(hot) {
  if (hot.wish === undefined || hot.build === undefined || hot.reloading) return;
  if (hot.failed.size > 0 && hot.builtAt < hot.failedAt) return;
  if ($hotWaits(hot).length === 0) {
    hot.reloading = true;
    location.reload();
  }
}
function $hotNote(hot, url, hash, id) {
  const [name, t] = $hotAddress(url);
  const before = hot.ran.get(name);
  if (before === undefined || t >= before.t) hot.ran.set(name, { hash, id, t });
  if (hot.failed.delete(name) && hot.wish === $hotFailure(name)) hot.wish = undefined;
  return [name, before !== undefined];
}
function $hotFailure(module) {
  return module + "'s update failed";
}
// An update of a module failed: its text does not link against what the page has, and none
// of the update's footers ran, or a module of it threw while it ran, after the footers of the
// modules run before it. The page has the module as it was, and it is left out of what the
// page waits for; the page is loaded again once the listing of an update after the failed
// one has what the rest of the page has run.
function $hotFailed(hot, module, at) {
  const t = typeof at === "number" ? at : hot.updating.get($hotPath(at)) ?? -1;
  hot.failed.add(module);
  hot.failedAt = Math.max(hot.failedAt, t);
  hot.wish ??= $hotFailure(module);
  $hotSettle(hot);
}
// The path of an address, as a server names the module that accepts an update.
function $hotPath(url) {
  const scheme = url.indexOf("//");
  const path = scheme < 0 ? url : url.slice(url.indexOf("/", scheme + 2));
  return path.split("?")[0];
}
// What a module that must not run again accepts its updates with: one that fails is told.
function $hotFailing(url) {
  return (updated) => { if (updated === undefined) $hotFailed($hotState(), $hotAddress(url)[0], url); };
}
// A module ran. One that a swap runs again gives its `$hot`, left for the swap's main.mjs;
// one that gives none must not run again, and wishes the page loaded again when it does.
function $hotRan(url, hash, id, init) {
  const hot = $hotState();
  const [name, again] = $hotNote(hot, url, hash, id);
  if (init !== undefined) {
    if (hot.booted) hot.left.push([name, init]);
  } else if (again) hot.wish ??= name + " ran again";
  $hotSettle(hot);
}
// main.mjs ran: the page has booted, or a swap has run, whose modules' `$hot` run in the
// microtask after it; a swap of more modules than a swap may be is a wish in their place. A
// page that booted with another text than the build lists loaded while a build was published.
function $hotBooted(url, hash, id) {
  const hot = $hotState();
  $hotNote(hot, url, hash, id);
  const left = hot.left;
  hot.left = [];
  if (left.length > (globalThis.__teqSwapUpTo ?? 100)) hot.wish ??= "a swap of " + left.length + " modules";
  else if (left.length > 0) queueMicrotask(() => {
    for (const [name, init] of left) {
      try { init(); } catch (e) { console.error("[teq] eager init failed in " + name + ".mjs", e); }
    }
  });
  if (!hot.booted) {
    hot.booted = true;
    if ($hotWaits(hot).length > 0) hot.wish ??= "the page loaded while a build was published";
  }
  $hotSettle(hot);
}
// hot-build.mjs ran: the listing of the build it ends. The one of the latest update counts.
function $hotBuild(url, id, build) {
  const hot = $hotState();
  const t = $hotAddress(url)[1];
  if (t < hot.builtAt) return;
  hot.builtAt = t;
  hot.build = build;
  hot.buildId = id;
  $hotSettle(hot);
}
// --hot: an object is reported under its qualified name once constructed, to the development
// server's runtime when the page has one, which looks for the component types among its fields.
function $hotObj(o, id) {
  globalThis.$teqHotObject?.(o, id);
  return o;
}
