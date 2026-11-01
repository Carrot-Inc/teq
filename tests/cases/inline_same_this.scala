// An inline method's `this` is elided where the call's prefix is the class's own `this` (`Inliner.canElideThis`): a
// plain `this`, an implicit one, `A.this`, an identity cast or ascription to `this.type`, an inline method calling
// another through one: the expansion's `s` in the constructor's code reads the parameter (3, what `A` got). A prefix
// widened to the class type (`asInstanceOf[A]`, `(this: A)`) is bound to a proxy (`A_this`), and its `s` reads the
// accessor, the subclass's override (2), as a method's read does after construction; so is a prefix cast or ascribed
// to an alias of `this.type` (`type Me = this.type`), whose type at dotty's decision is the alias, not the ThisType
// (2). A subclass's `this`, plain, implicit or cast to its `this.type`, is not `A.this.type` either and keeps the
// proxy too (2, the subclass's own accessor).
class A(val s: Int):
  inline def im: Int = s
  inline def im2: Int = im
  val plain = this.im
  val implicitReceiver = im
  val qualified = A.this.im
  val cast = this.asInstanceOf[this.type].im
  val castNested = this.asInstanceOf[this.type].im2
  val ascribed = (this: this.type).im
  val widened = this.asInstanceOf[A].im
  val widenedAscription = (this: A).im
  def inMethod = this.asInstanceOf[this.type].im
  type Me = this.type
  val aliased = this.asInstanceOf[Me].im
  val aliasAscribed = (this: Me).im
  val parenthesized = this.asInstanceOf[(this.type)].im
class B(override val s: Int) extends A(s + 1):
  val sub = this.im
  val subImplicit = im
  val subCast = this.asInstanceOf[this.type].im

@main def run(): Unit =
  val b = B(2)
  println(s"${b.plain} ${b.implicitReceiver} ${b.qualified}")
  println(s"${b.cast} ${b.castNested} ${b.ascribed}")
  println(s"${b.widened} ${b.widenedAscription} ${b.inMethod}")
  println(s"${b.sub} ${b.subImplicit} ${b.subCast}")
  println(s"${b.aliased} ${b.aliasAscribed} ${b.parenthesized}")
