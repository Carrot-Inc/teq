// An inline call's `this` prefix where teq's type of it is the class's own `this_type` in another form: a class with a
// self type calling its own inline method through a plain `this`, an implicit receiver, the self alias and `C.this`.
// dotty's prefix is the class's ThisType in each (elided: the constructor reads the parameter, 3), a generic class with
// a self type too; a subclass's `this` beside them keeps the proxy (the accessor, 2). A cast or an ascription to a self
// alias's `self.type`, which dotty's `findRef` types as the class's ThisType, is elided as `this.type` is (3), under a
// bare alias and a typed one alike; an alias of `this.type` keeps the proxy (2).
trait Marker
class S(val s: Int):
  self: Marker =>
  inline def im: Int = s
  val plain = this.im
  val implicitR = im
  val viaSelf = self.im
  val qualified = S.this.im
  def inMethod = this.im
class SS(override val s: Int) extends S(s + 1), Marker:
  val sub = this.im

class T(val s: Int):
  self =>
  type Me = this.type
  inline def im: Int = s
  val castSelf = this.asInstanceOf[self.type].im
  val ascribedSelf = (self: self.type).im
  val selfCast = self.asInstanceOf[self.type].im
  val selfAlias = (self: Me).im
class TS(override val s: Int) extends T(s + 1)

class U(val s: Int):
  self: Marker =>
  inline def im: Int = s
  val castSelf = this.asInstanceOf[self.type].im
  val ascribedSelf = (self: self.type).im
class US(override val s: Int) extends U(s + 1), Marker

trait Base:
  def s: Int
class SG[T](val s: Int):
  this: Base =>
  inline def im: Int = s
  val plain = this.im
class SGS(override val s: Int) extends SG[String](s + 1), Base

@main def run(): Unit =
  val ss = SS(2)
  println(s"self ${ss.plain} ${ss.implicitR} ${ss.viaSelf} ${ss.qualified} ${ss.inMethod} sub ${ss.sub}")
  println(s"generic self ${SGS(2).plain}")
  val ts = TS(2)
  println(s"self.type ${ts.castSelf} ${ts.ascribedSelf} ${ts.selfCast} ${ts.selfAlias}")
  val us = US(2)
  println(s"typed self.type ${us.castSelf} ${us.ascribedSelf}")
