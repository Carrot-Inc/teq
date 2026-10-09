// jars: fixtures fixtures-rdecl
// targets: js interp
// Not on the JVM, where link mode calls a default getter of the fixture's class file with the
// parameter before it, `RdMemo.$lessinit$greater$default$2(RdEv)`, which scalac's takes none
// of.
// The type variables a library body's patterns bind: one anonymous capture
// passed back to its own box against two distinct ones, a bounded capture, a named variable
// over a nested match, a named one beside an anonymous one, variables over a path, an
// object's own overloads called on a capture (`RdPick.this.put[_$8](b, b.get)`), and captures
// bounded as their patterns write them (`RdBox[? >: String]`), over a contravariant scrutinee
// and by classes named `Any` and `Nothing` that are not scala's.
import fix.rdecl.*
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.sameCapture(RdBox(3)))
  println(RdCalls.distinctCaptures(RdBox(3), RdBox("d")))
  println(RdCalls.bounded(RdFooBox(RdF1(4))))
  println(RdCalls.nested(RdBox("n")))
  println(RdCalls.namedAndAnon(RdBox(5), RdBox(6)))
  println(RdEvs.evaluate(RdEvs.program))
  println(RdEvs.evalNow(RdFn(RdMemo(RdNow(2)), (i: Int) => i.toString * 2)))
  println(RdPick.same(RdBox(9)))
  println(RdCalls.lowerBounded(RdBox[Any](1)))
  println(RdCalls.upperBounded(RdBox("four")))
  println(RdCalls.outerBound(RdBox(1), RdBox(2)))
  println(RdCalls.paramBound(RdBox[Any](1), 5))
  println(RdCalls.pathBound(new RdStart { type Start = Int; def start = 7 })(RdBox[Any](1)))
  println(RdCalls.contraLower(RdInBox[Any](1)))
  println(RdCalls.customUpper(RdBox(new RdCustom.Any(5))))
  println(RdCalls.customLower(RdBox[RdCustom.Any](new RdCustom.Any(1))))
