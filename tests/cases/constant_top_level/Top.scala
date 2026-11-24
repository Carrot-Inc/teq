// Top-level constants read from another file: an inline val of a singleton type, a final val inferred, a final val
// of a literal type.
package topcheck
final val t = 7
inline val b: t.type = t
final val f = 5
final val g: 6 = 6
