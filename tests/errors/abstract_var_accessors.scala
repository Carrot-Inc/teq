// An abstract var is a getter and a setter: a def or val alone implements the getter, a def with
// `override` cannot override either, and a var left undefined counts once.
// expect: 21:16: error: error overriding variable x in trait AbsVar of type Int;
// expect: method x_= of type (v: Int): Unit cannot override a mutable variable
// expect: 23:16: error: error overriding variable x in trait AbsVar of type Int;
// expect: method x of type Int cannot override a mutable variable
// expect: 26:19: error: object creation impossible, since var x_=(x$1: Int): Unit in trait AbsVar is not defined
// expect: 27:19: error: object creation impossible, since var x_=(x$1: Int): Unit in trait AbsVar is not defined
// expect: 28:19: error: object creation impossible, since var x: Int in trait AbsVar is not defined
// expect: 29:11: error: object creation impossible, since it has 3 unimplemented members: x, y, y_=
// expect: 30:20: error: object creation impossible, since def y_=(v: Int): Unit in trait DefPair is not defined
// expect: 30:38: error: error overriding method y in trait DefPair of type Int: variable y of type String has incompatible type
// expect: 8 errors found
trait AbsVar:
  var x: Int
trait DefPair:
  def y: Int
  def y_=(v: Int): Unit
class Setter extends AbsVar:
  def x = 1
  override def x_=(v: Int): Unit = ()
class Getter extends AbsVar:
  override def x = 1
  def x_=(v: Int): Unit = ()
object Main:
  val a: AbsVar = new AbsVar { def x = 1 }
  val b: AbsVar = new AbsVar { val x = 1 }
  val c: AbsVar = new AbsVar {}
  val d = new AbsVar with DefPair {}
  val e: DefPair = new DefPair { var y: String = "s" }
