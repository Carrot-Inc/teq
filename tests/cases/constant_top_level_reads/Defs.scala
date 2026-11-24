// A top-level val of a literal type and a final val read from another file: scalac folds a pure reference to a
// constant (constToLiteral's isPureExpr for the one, isIdempotentExpr for the other), so the reads run no
// initialiser of this file; the effectful val beside them shows whether it ran.
package tlreads
val p: 9 = 9
final val f = 5
val effect = { println("file init"); 0 }
