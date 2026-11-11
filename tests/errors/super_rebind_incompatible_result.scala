class X
class Y extends X
trait A[+T]:
  def f[U](x: U): T = null.asInstanceOf[T]
  def f(x: Int): Int = x
trait B extends A[X]:
  override def f[U](x: U): X = new X
trait C extends A[Y]:
  override def f[U](x: U): Y = new Y
  def g: Y = super.f[String]("x")
class Fail extends B with C
@main def run(): Unit = println(new Fail().g.isInstanceOf[Y])

// ResolveSuper.rebindSuper: matchingDenotation matches the polymorphic parameters,
// but B.f cannot implement C's super-accessor returning Y (scalac 3.8.4 E137).
// expect: super_rebind_incompatible_result.scala:11:7: error:
// expect: class Fail cannot be defined due to a conflict between its parents when
// expect: implementing a super-accessor for f in trait C:
// expect: is not a subtype of
// expect: 1 error found
