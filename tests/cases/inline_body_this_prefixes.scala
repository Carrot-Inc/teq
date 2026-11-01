// The inline `this` decision inside an inline method's body, the call made from the class's constructor: the inner
// call's prefix is the body's own cast or ascription, as dotty's `Inliner.canElideThis` reads it in the expansion. A
// cast or an ascription to `this.type` (or a self alias's `self.type`) is the ThisType (elided: the constructor reads
// the parameter, 3); one to an alias of it (`Me`) keeps the proxy (the accessor, the subclass's override, 2). teq's
// copy of the body keeps the written target's note (`InlineDefinition::written_this`), and a cast the definition kept
// unlowered moves its note to the `this` its lowering leaves. A subclass calling the same methods passes its own
// `this`, a proxy, so each inner call reads the accessor (2).
class A(val s: Int):
  self =>
  type Me = this.type
  inline def im: Int = s
  inline def im2: Int = im
  inline def viaAscThis: Int = (this: this.type).im
  inline def viaThis: Int = this.asInstanceOf[this.type].im
  inline def viaCastMe2: Int = this.asInstanceOf[Me].im2
  inline def viaAscMe: Int = (this: Me).im
  inline def viaSelf: Int = this.asInstanceOf[self.type].im
  inline def viaPlain: Int = this.im
  val b1 = viaAscThis
  val b2 = viaThis
  val b3 = viaCastMe2
  val b4 = viaAscMe
  val b5 = viaSelf
  val b6 = viaPlain
class B(override val s: Int) extends A(s + 1):
  val sub1 = viaThis
  val sub2 = viaAscThis
  val sub3 = viaAscMe

@main def run(): Unit =
  val b = B(2)
  println(s"body ${b.b1} ${b.b2} ${b.b3} ${b.b4} ${b.b5} ${b.b6}")
  println(s"sub ${b.sub1} ${b.sub2} ${b.sub3}")
