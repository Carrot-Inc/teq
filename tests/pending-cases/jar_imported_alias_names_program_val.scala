// jars: scala-library aliasimp-lib
// targets: jvm interp
// As jar_alias_names_program_val, with the library's alias a member of
// another object, imported and written as an identifier
// (tests/support/alias_imported_lib.scala: `import Kinds.Held` before
// the given class `Show[Held]`): the pickle names it by an identifier
// tree, whose read forces nothing. scalac 3.8.4 compiles the program;
// with the alias written `Kinds.Held`, a selection, it refuses it
// (E124). teq: "recursive value alias needs type". Moves to
// tests/classpath/jvm with the change that gives a pickle's alias its
// state while its right side is read.
package aliasimp

object Stock:
  val before = { import Counter.given; summon[Show[Int]].show }
  val alias = { import Counter.given; println("in:" + summon[Show[Int]].show); Shelf }

@main def run(): Unit = println(Stock.before + " use:" + Stock.alias.size)
