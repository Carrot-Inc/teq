// An object's `@threadUnsafe` lazy vals whose initialisers read themselves once (lazy_thread_unsafe_reentrant): a
// primitive, a reference and one that computes null.
import scala.annotation.threadUnsafe

object Fold:
  var depth = 0
  @threadUnsafe lazy val x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }
  var words = 0
  @threadUnsafe lazy val text: String = { words += 1; if words > 1 then "a" else text + "b" }
  var nulls = 0
  @threadUnsafe lazy val none: String = { nulls += 1; if nulls > 1 then null else { val inner = none; inner } }

@main def run(): Unit =
  println(s"x = ${Fold.x} after ${Fold.depth} run(s)")
  println(s"text = ${Fold.text} after ${Fold.words} run(s)")
  println(s"none = ${Fold.none} after ${Fold.nulls} run(s)")
  println(s"again: ${Fold.x} ${Fold.text} ${Fold.none} after ${Fold.depth}, ${Fold.words} and ${Fold.nulls} run(s)")
