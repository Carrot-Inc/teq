// Union and intersection laws, function and tuple subtyping, aliases, wildcards.
// expect: 28:60: error: type mismatch: found Cov[A | B], required Cov[A] | Cov[B]
// expect: 31:56: error: type mismatch: found Inv[A] | Inv[B], required Inv[A | B]
// expect: 34:42: error: value b is not a member of X1 | X2
// expect: 44:24: error: type mismatch: found Int => Int, required Any => Int
// expect: 49:25: error: type mismatch: found (Int, Any), required (Int, Int)
// expect: 69:42: error: type mismatch: found Inv[Nothing], required Inv[Int]
// expect: 74:26: error: type mismatch: found Int | String | Boolean, required Int | String
trait HasA { def a: Int }
trait HasB { def b: String }
trait Greets { def hello: String }
trait G1 extends Greets
trait G2 extends Greets
case class X1() extends HasA, HasB { def a = 1; def b = "x1" }
case class X2() extends HasA { def a = 2 }
case class GA() extends G1 { def hello = "ga" }
case class GB() extends G2 { def hello = "gb" }
class Cov[+T](val t: T)
class Con[-T]
class Inv[T]

object S:
  def commut[A, B](x: A | B): B | A = x
  def assoc[A, B, C](x: A | (B | C)): (A | B) | C = x
  def distr1[A, B, C](x: A & (B | C)): (A & B) | (A & C) = x
  def distr2[A, B, C](x: (A & B) | (A & C)): A & (B | C) = x
  def covUnion[A, B](x: Cov[A] | Cov[B]): Cov[A | B] = x
  def covUnionBack[A, B](x: Cov[A | B]): Cov[A] | Cov[B] = x       // scalac: error
  def conUnion[A, B](x: Con[A] | Con[B]): Con[A & B] = x
  def covInter[A, B](x: Cov[A] & Cov[B]): Cov[A & B] = x
  def invUnion[A, B](x: Inv[A] | Inv[B]): Inv[A | B] = x             // scalac: error
  def andMembers(x: HasA & HasB): String = x.a.toString + x.b
  def orMembers(x: GA | GB): String = x.hello                        // ok: member of the join Greets
  def orMembersBad(x: X1 | X2): String = x.b                         // scalac: error, b not in join HasA
  def orMembersA(x: X1 | X2): Int = x.a                              // ok
  def absorb[A, B <: A](x: A | B): A = x
  def absorb2[A, B <: A](x: A & B): B = x
  def andComm[A, B](x: A & B): B & A = x
  def unionToAny(x: Int | String): Any = x
  def interLeft[A, B](x: A & B): A = x
  def interRight[A, B](x: A & B): B = x
  def orInter[A, B, C](x: (A | B) & C): (A & C) | (B & C) = x
  val f1: Int => Any = (x: Any) => 1
  val f2: Any => Int = (x: Int) => 1                                 // scalac: error
  val f3: (Int, Int) => Int = (x: Any, y: Any) => 1
  val f4: Int => Int => Int = (x: Int) => (y: Any) => 1
  val t1: (Any, Any) = (1, "a")
  val t2: (Int, String) = (1, "a")
  val t3: (Int, Int) = ((1, "a"): (Int, Any))                        // scalac: error
  def bn(x: => Int): Int = x
  // a by-name function type `(=> Int) => Int` is outside the subset
  val bnf2: Int => Int = bn                                           // scalac: error?
  type IntList = List[Int]
  type Pair[A] = (A, A)
  val il: IntList = List(1)
  val il2: List[Int] = il
  val pr: Pair[Int] = (1, 2)
  val pr2: (Int, Int) = pr
  def wild(x: Option[?]): Boolean = x.isDefined
  val w1 = wild(Some(1))
  val w2: List[?] = List(1)
  val w3: Map[String, ?] = Map("a" -> 1)
  val w4: List[? <: AnyVal] = List(1)
  val w5: List[? <: AnyVal] = List("a")                              // scalac: error
  val w6: List[? >: Int] = List(1)
  def covIn[A](x: Cov[A]): Cov[Any] = x
  def conIn[A](x: Con[Any]): Con[A] = x
  def nothingIn(x: List[Nothing]): List[Int] = x
  def anyIn(x: Inv[Nothing]): Inv[Int] = x                            // scalac: error
  val u1: Int | String = 1
  val u2: Int | String | Boolean = u1
  val u3: (Int | String) | Boolean = u2
  val u4: Int | (String | Boolean) = u3
  val u5: Int | String = u4                                          // scalac: error

@main def main(): Unit =
  println(S.orMembers(GA()))
  println(S.andMembers(X1()))
  println(S.f1(1))
