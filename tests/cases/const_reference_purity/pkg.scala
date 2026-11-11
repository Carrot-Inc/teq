// A top-level val of a constant type read by its package's selection: FirstTransform's constToLiteral finds the
// selection no pure expression (the package is an idempotent path, the val no inline val), so its file runs.
package crp
val x: 42 = { println("init x"); 42 }
