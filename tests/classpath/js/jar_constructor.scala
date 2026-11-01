// jars: ctor-lib scala-library
// targets: js interp jvm
// A superclass from a jar whose val parameter a subclass overrides: the jar's class initialises its vals in its
// constructor, which reads its parameter for an identifier, in a closure, through an inline method, in a local
// function and its default (`Constructors.splitStats`, `intoConstr`; what the superclass got, 3) and the accessor for
// `this.s` (virtual, the subclass's 2); a jar class passing its parameter to its jar superclass passes the parameter
// (a super call reads no accessor), as scalac's class files do on the JVM.
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
