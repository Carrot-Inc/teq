// jars: inlinebodies-lib
// Inline methods of a jar scalac built (tests/support/inline_bodies_lib.scala), each expanded from
// the body the definition typed, read from the pickle and stored as a program's own definition is,
// as dotty expands the definition-typed body its `TreeUnpickler` reads (`Inlines.bodyToInline`):
// a constant argument is the parameter's path (`constValue[x.type]` of `value(3)` is 3); `codeOf`
// shows the expansion's typed argument, a constant folded, a by-value argument evaluated once into
// its proxy; each expansion's local class is a class of its own; an `erasedValue` an inline match
// tests is consumed; a generic class's match-type alias is seen from the receiver; a quote applying an
// inline parameter sees the argument's closure reduced (monocle's `Focus`). scalac prints the lines
// of the .expected file.
import inlinebodies.{Bodies, Counter, Focus, K0}
var effects = 0
case class P(a: Int, b: String)
def effect(n: Int): Int = { effects += 1; n }
@main def run(): Unit =
  println(Bodies.value(3))
  val three: 3 = Bodies.value(3)
  println(three + 1)
  println(Bodies.code(1 + 2))
  println(Bodies.codeInline(effect(4) * 2))
  println(Bodies.code2(5))
  println(Bodies.proxied(effect(6)))
  println(effects)
  val a = Bodies.make(3)
  val b = Bodies.make(4)
  println(s"${a()},${b()}")
  println(Bodies.made(1) + " " + Bodies.made(2))
  println(Bodies.kind)
  println(s"${Bodies.kindOf[Int]},${Bodies.kindOf[String]},${Bodies.kindOf[Boolean]}")
  println(new Counter(10).plus(2))
  println(K0.count[Option, P])
  println(Focus[P](_.a))
  println(Bodies.value({ def y: Int = 3; 5 }))
