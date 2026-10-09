// The erased bound of a type pattern over an abstract type follows an abstract constituent to
// its own bound's erasure before a union's erased lub or an intersection's erased glb is taken
// (TypeErasure.scala 767-769, 773-779, then 806-819): `T <: A | Int` with `A <: String` tests
// `Object`, `T <: A & Base` with `A <: Marker` tests `Base`; a chain of bounds, an opaque bound,
// a match type's, an applied class's, a value class's union, `Null` and `Nothing` parts, an enum.
trait Marker
class Base
class Child extends Base with Marker
class Box[+A](val a: A)
class VC(val n: Int) extends AnyVal
object O:
  opaque type Num = Int
  def num: Num = 1
  opaque type Lit = 1
  def lit: Lit = 1
  opaque type Choice = String | Int
  def choice: Choice = "s"
enum E:
  case A, B
type MT[X] = X match
  case Int => 1
  case _ => String
object Main:
  def chain[A <: 1, B <: A](x: Any) = x match { case _: B @unchecked => true; case _ => false }
  def opaque[T <: O.Lit](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def opaqueUnion[T <: O.Choice](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def mt[T <: MT[Int]](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def box[T <: Box[String]](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def inter[T <: Marker & Base](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def union[T <: VC | String](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def nullable[T <: String | Null](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def bottom[T <: String | Nothing](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def en[T <: E](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def abstractUnion[A <: String, T <: A | Int](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def abstractInter[A <: Marker, T <: A & Base](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def nested[A <: Marker, B <: A | Base, T <: B & Base](x: Any) = x match { case _: T @unchecked => true; case _ => false }
  def main(args: Array[String]): Unit =
    println("chain=" + chain[1, 1](2))
    println("opaque=" + opaque[O.Lit](2))
    println("opaque-union=" + opaqueUnion[O.Choice](new Base))
    println("match=" + mt[1](2))
    println("box=" + box[Box[String]](new Box(1)))
    println("inter=" + inter[Child](new Base))
    println("union=" + union[String](new Base))
    println("null=" + nullable[String](null))
    println("bottom=" + bottom[String]("s"))
    println("enum=" + en[E](E.B))
    println("abstract-union=" + abstractUnion[String, Int](new Base))
    println("abstract-union-string=" + abstractUnion[String, Int]("s"))
    println("abstract-inter=" + abstractInter[Marker, Child](new Base))
    println("abstract-inter-string=" + abstractInter[Marker, Child]("s"))
    println("nested=" + nested[Marker, Base, Child](new Base))
    println("nested-null=" + nested[Marker, Base, Child](null))
