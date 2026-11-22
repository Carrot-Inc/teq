// A list a trait of evidence alone has no clause for, as its constructor is normalized (an empty clause before a
// leading implicit one, after using clauses alone): E050, as scalac refuses it, by a parent and by an anonymous class.
// expect: 23:18: error: constructor Ti in trait Ti does not take more parameters
// expect: 24:18: error: constructor Ti in trait Ti does not take more parameters
// expect: 25:18: error: constructor Tu in trait Tu does not take more parameters
// expect: 26:18: error: constructor Tu2 in trait Tu2 does not take more parameters
// expect: 27:18: error: constructor Tu in trait Tu does not take more parameters
// expect: 29:16: error: constructor Ti in trait Ti does not take more parameters
// expect: 30:16: error: constructor Ti in trait Ti does not take more parameters
// expect: 31:16: error: constructor Tu in trait Tu does not take more parameters
// expect: 32:16: error: constructor Tu2 in trait Tu2 does not take more parameters
// expect: 33:16: error: constructor Tu in trait Tu does not take more parameters
class A(val s: String)
class B(val s: String)
given A = A("ga")
given B = B("gb")
trait Ti(implicit a: A, b: B):
  def show: String = a.s + " " + b.s
trait Tu(using a: A, b: B):
  def show: String = a.s + " " + b.s
trait Tu2(using a: A)(using b: B):
  def show: String = a.s + " " + b.s
class X0 extends Ti(using A("wa"), B("wb"))()
class X1 extends Ti()(using A("wa"), B("wb"))()
class X3 extends Tu(using A("wa"), B("wb"))()()
class X4 extends Tu2(using A("wa"))()(using B("wb"))
class X5 extends Tu()(using A("wa"), B("wb"))
object O:
  val y0 = new Ti(using A("wa"), B("wb"))() {}
  val y1 = new Ti()(using A("wa"), B("wb"))() {}
  val y3 = new Tu(using A("wa"), B("wb"))()() {}
  val y4 = new Tu2(using A("wa"))()(using B("wb")) {}
  val y5 = new Tu()(using A("wa"), B("wb")) {}
