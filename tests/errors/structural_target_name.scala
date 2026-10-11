// A member erasure knows by another name (`@targetName`) holds no structural refinement of its
// source name, an alternative of an overloaded name or not (dotty's hasMatchingMember: qualifies'
// hasTargetName); the alternative that keeps its name does.
// expect: 18:38: error: type mismatch: found A, required AnyRef{def f(x: Int): String}
// expect: 19:38: error: type mismatch: found B, required AnyRef{def f(x: Int): String}
// expect: 2 errors found
import scala.annotation.targetName
class A:
  @targetName("g") def f(x: Int): String = "i"
  def f(x: String): String = "s"
class B:
  @targetName("g") def f(x: Int): String = "i"
class C:
  @targetName("g") def f(x: Int): String = "i"
  def f(x: String): String = "s"
  def f(x: Long): String = "l"
object Main:
  val a: { def f(x: Int): String } = new A
  val b: { def f(x: Int): String } = new B
  val c: { def f(x: String): String } = new C
