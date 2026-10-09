// Every call of `make` runs the quote and copies its anonymous class. A copy is named by the
// call that made it, so that typing this file again (tests/split-watch.sh) names the copies as a
// build from nothing does.
package copies.use

import copies.macros.*

object Use:
  def first(): String = Macros.make("a").show
  def second(): String = Macros.make("b").show + Macros.make("c").show

inline def twice(inline s: String): String = Macros.make(s).show + Macros.make(s).show

@main def run(): Unit =
  println(Use.first())
  println(Use.second())
  println(twice("d"))
