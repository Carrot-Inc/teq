// The reachability of type tests in an open world (dotty's `Space.intersectUnrelatedAtomicTypes`,
// `TypeComparer.provablyDisjoint`): a trait tested over a class that is not final is reachable,
// a subclass may mix it in, whether the program has one or not (over a final class, or a class
// over an unrelated one, the test is scalac's E030 error); a class unrelated to a part of an
// intersection scrutinee, or of an abstract one's bound, is unreachable.
class C
trait T
trait U
class D extends C with T
sealed trait S
case class S1() extends S
case class S2() extends S
class A
class B

def open(x: C): Int = x match { case _: T => 1; case _ => 0 }
def withChild(x: C): Int = x match { case _: T => 1; case _: D => 2; case _ => 0 }
def string(x: CharSequence): Int = x match { case _: String => 1; case _ => 0 }
def traits(x: T): Int = x match { case _: U => 1; case _ => 0 }
def sealedTrait(x: S): Int = x match { case _: T => 1; case S1() => 2; case S2() => 3 }
def inter(x: A & T): Int = x match { case _: B => 1; case _ => 0 }
def interOk(x: C & T): Int = x match { case _: D => 1; case _ => 0 }
def bound[X <: A & T](x: X): Int = x match { case _: B => 1; case _ => 0 }
def unionBound[X <: A | B](x: X): Int = x match { case _: C => 1; case _ => 0 }
def openBound[X <: C](x: X): Int = x match { case _: T => 1; case _ => 0 }
trait Holder:
  type M <: A & T
  def member(x: M): Int = x match { case _: B => 1; case _ => 0 }

@main def run(): Unit =
  println(open(new D) + withChild(new C) + string("s") + traits(new T {}))
  println(sealedTrait(S1()) + inter(new A with T) + interOk(new D))
