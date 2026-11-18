// jars: library-object-vals-lib
// std: lean scala-library
// targets: js interp jvm
// An object of a library's classes is loaded where it is named, and its load runs what scalac's runs: its own
// vals' and vars' initialisers and statements and those of its bases, in their order, whether or not a member
// is read; a lazy val and a given run at their first read. The expectation is scalac 3.8.4's.
import valslib.*

@main def run(): Unit =
  println("01 its own val"); val a: Any = OwnVal
  println("02 its own private val"); val b: Any = OwnPrivateVal
  println("03 its own var"); val c: Any = OwnVar
  println("04 its own pattern val"); val d: Any = OwnPattern
  println("05 its own statement"); val e: Any = OwnStatement
  println("06 its own lazy val"); val f: Any = OwnLazy
  println("07 a trait's val"); val g: Any = TraitVal
  println("08 a trait's private val"); val h: Any = TraitPrivateVal
  println("09 a trait's var"); val i: Any = TraitVar
  println("10 a trait's statement"); val j: Any = TraitStatement
  println("11 a class's val"); val k: Any = ClassVal
  println("12 a class's statement"); val l: Any = ClassStatement
  println("13 a trait's val two levels up"); val m: Any = TraitUp
  println("14 an overriding val"); val n: Any = Overrider
  println("15 a trait's argument"); val o: Any = TraitParam
  println("16 an enum's val"); val p: Any = Shade.Dark
  println("17 a given"); val q: Any = GivenHolder
  println("18 its own val, read after the load"); val r: Any = LateRead; println("  named"); println("  " + LateRead.v)
  println("19 a trait's val, read after the load"); val t: Any = LateTraitRead; println("  named"); println("  " + LateTraitRead.held)
  println("end")
