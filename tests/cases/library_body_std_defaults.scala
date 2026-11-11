// jars: stddefaults-lib
// A library's bodies that leave out the defaults of the standard library's members: scalac passes
// each as the getter's application on the receiver, which runs the default of the member the
// receiver's class has (an iterator's `indexWhere` on the iterator `flatMap` makes).
import stddefaultslib.Scan

@main def run(): Unit =
  println(Scan.firstMatch(Some("a b c"), "b"))
  println(Scan.firstMatch(None, "b"))
  println(Scan.matchFrom(Iterator(4, 5, 6), 6))
  println(Scan.positionOf(Seq(3, 1, 4, 1), 1))
  println(Scan.firstOver(Vector(1, 5, 9), 4))
  println(Scan.lastBelow(List(1, 5, 2, 8), 3))
  println(Scan.begins(List(1, 2, 3), List(1, 2)))
  println(Scan.firstUpper("abCd"))
  println(Scan.windows(List(1, 2, 3)))
  println(Scan.counted(3))
  println(Scan.ranged(4))
