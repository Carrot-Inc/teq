// jars: ctor-lib scala-library
// targets: js
// teq: --release
// tests/classpath/js/jar_constructor.scala under `--release`: the jar's classes initialise their vals in their
// constructors, which read their parameters (3) and the accessor for `this.s` (2), a super call the parameter.
class B(override val s: Int) extends ctorlib.A(s + 1)
class C(override val s: Int) extends ctorlib.Mid(s + 1)

@main def run(): Unit =
  val b = B(2)
  println(b.ident)
  println(b.closure())
  println(b.selected)
  println(b.s)
  println(s"${b.viaInline} ${b.viaLocal} ${b.viaDefault}")
  val c = C(2)
  println(s"${c.b} ${c.m} ${c.s}")
