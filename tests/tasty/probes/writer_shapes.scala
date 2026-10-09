// Shapes the writer met early: an export of an inherited generic member and of
// types alone, an opaque type with a bound, stackable `abstract override`, a case class whose
// companion overloads `apply` with the generated one's arity, a class with a symbolic name,
// a file of type aliases alone.
package probe.writershapes

class Impl[A]:
  def id(a: A): A = a
  def narrow[B <: A](x: B): B = x
object Strings extends Impl[String]
object Api:
  export Strings.{id, narrow}

class IA[X]:
  def a(x: X): X = x
object WA extends IA[String]
class IB[Y]:
  def b(y: Y): Y = y
object WB extends IB[Int]
object Wild:
  export WA.*
  export WB.*

object Kinds:
  class C
  type T = Int
  type Box[X] = List[X]
object TypesOnly:
  export Kinds.C
  export Kinds.{T, Box}

object Ids:
  opaque type Name <: String = String
  opaque type Pos >: Nothing <: Int = Int

trait Base:
  def f: Int
class Concrete extends Base:
  def f: Int = 1
trait Plus extends Base:
  abstract override def f: Int = super.f + 1

case class C(x: Int)
object C:
  def apply(s: String): C = new C(s.length)
case class D(x: Int)
object D:
  def apply(x: Int): D = new D(x + 1)

class +(val x: Int)
