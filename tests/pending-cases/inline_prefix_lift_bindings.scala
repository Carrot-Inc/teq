// dotty's `Inlines.inlineCall` lifts the statements of a block the call's prefix is, and an `Inlined` prefix's
// bindings, out before the call (`liftBindings`), stopping at a plain method's body, ascribed its result type
// (`PrepareInlineable.wrapRHS`'s `Typed`); `canElideThis` then reads the type of what is left, and an elided prefix is
// not evaluated (`registerType` makes no proxy where `canElideThis` holds). scalac: `{ println("a"); this }` and
// `{ val k = 1; this }` are the ThisType (3, "a" printed); a call of a method of the class's type keeps the proxy (2,
// "arg 5" printed); one of `this.type` is the ThisType, its body's `println` never run (3, no "argT 6"); a
// subclass's `this` keeps the proxy (2). teq binds a prefix with statements to the proxy, evaluating it
// (`2 2 2 2 2 7 sub 2 2`, the same on 68031d1913); a prefix that is a block without statements is its value's type
// (tests/classpath/js/jar_alias_this.scala's transparent result). Waiting on `liftBindings`, a mechanism of its own.
object Obj:
  val o = 7
  inline def om: Int = o
class A(val s: Int):
  inline def im: Int = s
  inline def add(inline k: Int): Int = s + k
  inline def withArg(k: Int): A = { println(s"arg $k"); this }
  inline def withArgT(k: Int): this.type = { println(s"argT $k"); this }
  inline def viaBlock: Int = { println("v"); this }.im
  val b1 = { println("a"); this }.add({ println("b"); 0 })
  val b2 = { val k = 1; this }.im
  val b3 = this.withArg(5).im
  val b4 = this.withArgT(6).im
  val b5 = viaBlock
  val b6 = { println("o"); Obj }.om
class B(override val s: Int) extends A(s + 1):
  val c1 = { println("c"); this }.im
  val c2 = this.withArgT(8).im

@main def run(): Unit =
  val b = B(2)
  println(s"lift ${b.b1} ${b.b2} ${b.b3} ${b.b4} ${b.b5} ${b.b6} sub ${b.c1} ${b.c2}")
