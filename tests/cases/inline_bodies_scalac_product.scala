// jars: inlinebodies-lib
// Inline methods of a jar scalac built (tests/support/inline_bodies_lib.scala), each expanded from
// the body the definition typed, read from the pickle and stored as a program's own definition is,
// as dotty expands the definition-typed body its `TreeUnpickler` reads (`Inlines.bodyToInline`):
// a constant argument is the parameter's path (`constValue[x.type]` of `value(3)` is 3); `codeOf`
// shows the expansion's typed argument, a constant folded, a by-value argument evaluated once into
// its proxy; each expansion's local class is a class of its own; an `erasedValue` an inline match
// tests is consumed; a generic class's match-type alias is seen from the receiver; a quote applying an
// inline parameter sees the argument's closure reduced (monocle's `Focus`), a curried one at each list, an
// effect bound once (`BetaReduce`); a lazy val's path binds its singleton, its initialiser run once; a local
// class constructs through its copied secondary constructors; a type pattern over the abstract type
// the scrutinee has tests for null (`testNotNull`); a class's by-name members stay by-name in its copies.
// scalac prints the lines of the .expected file.
import inlinebodies.{Abstract, Bodies, ByName, Counter, Ctors, Focus, K0, Quoted, Reduce, Tag, Tests}
var effects = 0
case class P(a: Int, b: String)
def effect(n: Int): Int = { effects += 1; n }
def genCheck[T](d: Any): Boolean = Abstract.check[T](d)
def genLazy[T](d: Any): Option[T] = Abstract.lazyCheck[T](d)()
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
  println(Quoted.value(3, x => x + 1))
  println(Quoted.code(1 + 2, x => x + 1))
  println(Quoted.shape(x => x + 1))
  println(Tag.derived(3))
  println(Tag.derived(300))
  println(Bodies.value({ val f: Int => Int = x => x + 1; 3 }))
  lazy val l: 3 = { println("init"); 3 }
  println(Bodies.both(l))
  println(Bodies.both(l))
  println(Reduce.curried(x => y => x + y))
  println(Reduce.folded(x => x + 3))
  println(Reduce.runCurried(x => y => x + y))
  println(Reduce.runEffect(x => { println("b"); x + x }))
  println(Ctors.make(7))
  println(Ctors.make2(4))
  val none: String = null
  println(s"${Tests.matches[String](none, true)},${Tests.matches[String]("s", true)},${Tests.matches[Int](3, true)}")
  println(s"${Tests.tester[String](true)(none)},${Tests.tester[String](true)("s")}")
  println(ByName.derive(3))
  println(ByName.derive2("s"))
  println(s"${genCheck[String](null)},${genCheck[String](1)},${genLazy[String](null)},${genLazy[String]("s")}")
