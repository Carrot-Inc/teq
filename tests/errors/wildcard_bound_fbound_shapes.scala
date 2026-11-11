// expect: 29:22: error: type argument X does not conform to upper bound BS[BS[?, X]#T, X]
// expect: 30:31: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 32:33: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 33:20: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 34:41: error: type argument S does not conform to upper bound BS[Int, S]
// expect: 36:39: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 37:31: error: type argument Int does not conform to upper bound BS[?, Int]
// expect: 40:12: error: type argument String does not conform to upper bound BS[?, String]
// expect: 42:14: error: type argument ? >: String does not overlap with upper bound Comparable[C8[? >: String]#T]
// expect: 44:47: error: type argument B does not conform to upper bound M9[B, M9[?, B]#A]
// expect: 45:20: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 45:44: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 46:16: error: type argument IS does not conform to upper bound BS[BS[?, IS]#T, IS]
// expect: 47:23: error: type argument IS does not conform to upper bound BS[BS[? <: Int, IS]#T, IS]
// expect: 49:26: error: type argument List[Option[String]] does not conform to upper bound List[Option[D12[? <: Int, List[Option[String]]]#A]]
// expect: 50:26: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 52:25: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 55:27: error: type argument H15.this.S does not conform to upper bound BS[BS[? <: Int, H15.this.S]#T, H15.this.S]
// expect: 56:41: error: type argument S does not conform to upper bound BS[BS[?, S]#T, S]
// expect: 19 errors found
// `TypeOps.boundsViolations` (TypeOps.scala 606-740) over F-bounded parameters, as scalac 3.8.4 reports each (one
// column before teq's, its wildcard ` >: String` teq's `? >: String`): `skolemizeWildcardArgs`' skolem `BS[?, S]#T` through
// an alias's bound, an alias of the class, an alias's right side, an `extends` clause, a given, a higher-kinded
// and an abstract member's `LambdaParam` bound, a Java class's F-bound, mutual F-bounds, a refinement over the
// bound, a value's type, a nested application, a type lambda's body, a context bound, an abstract member's F-bound,
// a tuple. The closure adversary of the pass on 5bd85bcd1 (round 23).
trait BS[T, S <: BS[T, S]]
trait IS extends BS[Int, IS]
type Self[X] = BS[?, X]
def a1[S <: Self[S]](x: BS[?, S]): Int = 1
type BSA[T, S <: BS[T, S]] = BS[T, S]
def a2[S <: BS[?, S]](x: BSA[?, S]): Int = 1
type W3[S] = BS[?, S]
trait E4[S <: BS[?, S]] extends BS[Int, S]
trait Conv[A]
given g5[S <: BS[Int, S]]: Conv[BS[?, S]] = ???
def a6[F[X <: BS[?, X]]](x: F[Int]): Int = 1
trait T7:
  type M[X <: BS[?, X]]
  def g: M[String]
class C8[T <: Comparable[T]]
type X8 = C8[? >: String]
class M9[A <: M9[A, B], B <: M9[B, A]]
def a9[A <: M9[A, B], B <: M9[B, A]](x: M9[?, B]): Int = 1
def a10[S <: BS[?, S] { type U }](x: BS[?, S]): Int = 1
val v11: BS[?, IS] = ???
val u11: BS[? <: Int, IS] = ???
class D12[A, B <: List[Option[A]]]
type X12 = D12[? <: Int, List[Option[String]]]
type L13 = [S] =>> BS[?, S]
trait TC[A]
def a14[S: TC](x: BS[?, S]): Int = 1
trait H15:
  type S <: BS[?, S]
  type Bad = BS[? <: Int, S]
def a16[S <: BS[Int, S]](x: (Int, BS[?, S])): Int = 1
