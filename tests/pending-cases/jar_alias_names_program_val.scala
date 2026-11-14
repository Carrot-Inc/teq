// jars: scala-library aliasval-lib
// targets: jvm interp
// The program defines again an object its library was compiled against
// (tests/support/alias_val_lib.scala's `Stock`), and the library's given
// class is typed through the library's own alias of that object's val
// (`type Held = Stock.alias.type; given heldShow: Show[Held] with`). An
// earlier val searches first: the search compares `heldShow`, reads the
// alias, whose right side asks `alias`, and `alias`'s own search then
// meets the alias while its right side is being read. scalac 3.8.4 gives
// an alias empty bounds until its right side is read
// (TreeUnpickler.scala:982-983), the candidate does not fit there, and
// the program compiles; with the given typed by the val directly
// (`Show[Stock.alias.type]`) scalac refuses it (E124). teq:
// "recursive value alias needs type". Moves to tests/classpath/jvm with
// the change that gives a pickle's alias that state.
package aliasval

object Stock:
  val before = { import Counter.given; summon[Show[Int]].show }
  val alias = { import Counter.given; println("in:" + summon[Show[Int]].show); Shelf }

@main def run(): Unit = println(Stock.before + " use:" + Stock.alias.size)
