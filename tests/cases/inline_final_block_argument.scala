//> using platform js
// An inline call's argument as the inliner reads it, before FirstTransform folds a constant reference: a block
// ending in a reference binds a proxy of the widened type, whose constant `constValueOpt` does not find, whether the
// val is final or not; a reference alone is a path, bound as its singleton type.
// On Scala.js only; tests/errors/inline_parameter_block_path_final.scala
// holds the binding for every target.
import scala.compiletime.constValueOpt
inline def value(x: Int): Int = constValueOpt[x.type].getOrElse(-1)
final val b: 9 = 9
val a: 8 = 8
object O:
  final val c: 7 = 7
@main def m(): Unit =
  println(value({ def unused = 0; b }))
  println(value({ def unused = 0; a }))
  println(value({ def unused = 0; O.c }))
  println(value(b))
