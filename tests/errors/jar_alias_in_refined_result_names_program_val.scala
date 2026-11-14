// jars: scala-library aliasgiven-lib
// expect: 21:60: error:
// expect: heldShow needs
// expect: 1 error found
// The program defines again an object its library was compiled against
// (tests/support/alias_given_lib.scala's `aliasrefined.Stock`), and the
// library's given has a refined result typed through the library's own
// alias of that object's val (`type Held = Stock.alias.type; given
// heldShow: (R { type T = Held }) = ...`). An
// earlier val searches first: scalac 3.8.4 computes the signature of
// `heldShow`, reads its definition in the jar, computes the signature of
// `Held`, whose right side asks `alias`, whose own search then meets
// `heldShow`'s signature in progress: E124 at that search ("given
// instance heldShow needs result type because its right-hand side
// attempts implicit search"). teq accepts the program: it reads a val
// that a pickle names as its path and does not ask the val.
package aliasrefined

object Stock:
  val before = { import Counter.given; summon[Show[Int]].show }
  val alias = { import Counter.given; println("in:" + summon[Show[Int]].show); Shelf }

@main def run(): Unit = println(Stock.before + " use:" + Stock.alias.size)
