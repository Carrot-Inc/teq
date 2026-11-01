// A cast or an ascription of `this` to `this.type` is the ThisType (elided: the constructor reads the parameter, 3),
// one to an alias of it keeps the proxy (the accessor, the subclass's override, 2), and a subclass's `this` keeps it
// too (2), the inline call standing in a macro's argument that the macro's quote splices back. An inline method's
// `this` result is a prefix of its result type (`PrepareInlineable.wrapRHS`): the class's keeps the proxy (2),
// `this.type` is the ThisType (3).
class A(val s: Int):
  type Me = this.type
  inline def im: Int = s
  val q1 = Mac.keep(this.asInstanceOf[this.type].im)
  val q2 = Mac.keep(this.asInstanceOf[Me].im)
  val q3 = Mac.keep((this: Me).im)
  val q4 = Mac.keep((this: this.type).im)
  inline def self: A = this
  inline def selfT: this.type = this
  val q6 = Mac.keep(this.self.im)
  val q7 = Mac.keep(this.selfT.im)
class B(override val s: Int) extends A(s + 1):
  val q5 = Mac.keep(this.im)

@main def run(): Unit =
  val b = B(2)
  println(s"quoted ${b.q1} ${b.q2} ${b.q3} ${b.q4} sub ${b.q5} results ${b.q6} ${b.q7}")
