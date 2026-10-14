// A class parameter with a modifier of its own (`implicit`, `private`, `protected`, `final`, `override`) needs
// `val` or `var`, as dotty's `param` of a class asks for one; `implicit val` makes the accessor a given of the
// body, a clause's leading `implicit` the clause's parameters.
class C(x: Int, implicit y: String):
  def get: String = summon[String]
case class A(private x: Int)
class B(protected y: Int, final z: Int)
class D(val w: Int, implicit val v: String)(implicit u: Int)
// expect: 4:26: error: `val` or `var` expected
// expect: 6:22: error: `val` or `var` expected
// expect: 7:19: error: `val` or `var` expected
// expect: 7:33: error: `val` or `var` expected
