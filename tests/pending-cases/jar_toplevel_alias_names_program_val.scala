// jars: scala-library aliastop-lib
// targets: jvm interp
// As jar_alias_names_program_val, with the library's alias at the top
// level of its package (tests/support/alias_top_lib.scala: `type Held =
// Stock.alias.type` beside `object Counter`, whose given class is
// `Show[Held]`). The program defines `Stock` again and an earlier val
// searches first. scalac 3.8.4 compiles it: the alias has empty bounds
// while its right side is read (TreeUnpickler.scala:982-984), where the
// val is asked. teq: "recursive value alias needs type". Moves to
// tests/classpath/jvm with the change that gives a pickle's alias that
// state.
package aliastop

object Stock:
  val before = { import Counter.given; summon[Show[Int]].show }
  val alias = { import Counter.given; println("in:" + summon[Show[Int]].show); Shelf }

@main def run(): Unit = println(Stock.before + " use:" + Stock.alias.size)
