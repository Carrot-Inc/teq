// A bound naming another parameter whose argument is a wildcard is met where the wildcard's skolem `<app>#A`
// (`argForParam`, Types.scala 2794) is known to lie: `? >: Int` for `B <: A` with `A := ? >: Int` (below the skolem
// what is below its lower end), `? <: Int` against `? <: Int`, and for variant parameters, whose wildcard stands
// for its upper (covariant) or lower (contravariant) end (`C[+A, B <: A]` with `C[? >: Int, String]`). scalac 3.8.4
// accepts all of it.
class Up[A, B <: A](val n: Int)
class Lo[A, B >: A](val n: Int)
class CoUp[+A, B <: A](val n: Int)
class CtLo[-A, B >: A](val n: Int)
class CtUp[-A, B <: A](val n: Int)
class CoList[+A, B <: List[A]](val n: Int)
class Outer[X]:
  class In[T <: X](val n: Int)
  def mk: In[? >: X] = In[X](9)
type P = Up[CharSequence, ? >: String]
type Q = Up[? >: Int, ? >: Int]
type R = Up[? <: Int, ? <: Int]
type S = CoUp[? >: Int, String]
type T = CtLo[? <: Int, String]
type U = CoUp[? <: Int, Int]
type V = CtUp[? >: Int, Int]
type W = CtLo[? >: Int, Int]
type Z = CoList[? <: Int, List[Int]]
@main def main(): Unit =
  val p: P = Up[CharSequence, String](1)
  val q: Q = Up[Int, Int](2)
  val r: R = Up[Int, Int](3)
  val s: S = CoUp[Any, String](4)
  val t: T = CtLo[Nothing, String](5)
  val u: U = CoUp[Int, Int](6)
  val v: V = CtUp[Int, Int](7)
  val w: W = CtLo[Int, Int](8)
  val z: Z = CoList[Int, List[Int]](10)
  println(List(p.n, q.n, r.n, s.n, t.n, u.n, v.n, w.n, Outer[Int]().mk.n, z.n).mkString(","))
