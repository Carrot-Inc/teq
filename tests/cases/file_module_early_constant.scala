//> using platform jvm
// split-watch: jvm
// A top-level final val whose initialiser is a block with an effect, of the constant type `42` as dotty's typer records
// it: dotty's `isConstExprFinalVal` asks its result type, so the getter answers 42 even before the file module's
// initialiser has run the block, which it still runs for its effect. Read by reflection during that initialiser.
package early

import scala.reflect.Selectable.reflectiveSelectable

val before = Class.forName("early.file_module_early_constant$package$").getDeclaredField("MODULE$").get(null)
  .asInstanceOf[{ def answer: Int }].answer
final val answer = { println("effect"); 42 }

@main def run(): Unit =
  println(before)
  println(answer)
