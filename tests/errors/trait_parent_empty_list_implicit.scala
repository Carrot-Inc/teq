// An empty list given to a trait's implicit clause (the normalization's empty clause taken by the first `()`): E171,
// as scalac refuses it, by a parent and by an anonymous class.
// expect: 13:18: error: missing argument for parameter a of constructor Ti in trait Ti: (implicit a: A, b: B): Ti
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect: 15:16: error: missing argument for parameter a of constructor Ti in trait Ti: (implicit a: A, b: B): Ti
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
class A(val s: String)
class B(val s: String)
given A = A("ga")
given B = B("gb")
trait Ti(implicit a: A, b: B):
  def show: String = a.s + " " + b.s
class X2 extends Ti()()
object O:
  val y2 = new Ti()() {}
