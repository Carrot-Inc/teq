// Soft unions when type variables are instantiated.
// expect: 23:35: error: type mismatch: found List[Shape], required List[Circle | Square]
// expect: 29:42: error: type mismatch: found List[Option[Int]], required List[Some[Int] | None]
// expect: 33:29: error: type mismatch: found Shape, required Circle | Square
// expect: 39:35: error: type mismatch: found Cov[Shape], required Cov[Circle | Square]
// expect: 42:35: error: type mismatch: found List[Shape], required List[Circle | Square]
// expect: 44:35: error: type mismatch: found List[Shape], required List[Circle | Square]
// expect: 50:39: error: type mismatch: found Option[Shape], required Option[Circle | Square]
trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class A(x: Int)
case class B(y: Int)
class Inv[T](val t: T)
class Cov[+T](val t: T)

object U:
  val c = true
  def same[T](a: T, b: T): T = a
  val l1 = List(1, "a")
  val m1: List[Int | String] = l1               // ok: join Matchable transparent
  val l2 = List(Circle(1), Square(2))
  val m2: List[Circle | Square] = l2            // scalac: error, List[Shape]
  val m2b: List[Shape] = l2                     // ok
  val l3 = List(A(1), B(2))
  val m3: List[A | B] = l3                      // ok
  val l4 = List(Some(1), None)
  val m4: List[Option[Int]] = l4                // ok
  val m4b: List[Some[Int] | None.type] = l4     // scalac: error
  val s1 = same(1, "a")
  val t1: Int | String = s1                     // ok
  val s2 = same(Circle(1), Square(2))
  val t2: Circle | Square = s2                  // scalac: error, Shape
  val i1 = Inv(if c then Circle(1) else Square(2))
  val j1: Inv[Shape] = i1                       // ok: soft union widened to join inside invariant Inv
  val i2 = Inv(if c then 1 else "a")
  val j2: Inv[Int | String] = i2                // ok: union kept
  val k1 = Cov(if c then Circle(1) else Square(2))
  val k1b: Cov[Circle | Square] = k1            // scalac: error (Cov[Shape])
  val hard: Circle | Square = Circle(1)
  val l5 = List(hard, Square(2))
  val m5: List[Circle | Square] = l5            // ok? lub of hard union and Square
  val l6 = List(Circle(1), Square(2), hard)
  val m6: List[Circle | Square] = l6            // ok?
  val l7 = List(1, 2L)
  val m7: List[Long] = l7                       // ok: harmonised
  val l8 = List(1, "a", 2.0)
  val m8: List[Int | String | Double] = l8      // ok?
  val opt = Option(if c then Circle(1) else Square(2))
  val opt2: Option[Circle | Square] = opt       // scalac: error

@main def main(): Unit =
  println(U.l1)
  println(U.s1)
