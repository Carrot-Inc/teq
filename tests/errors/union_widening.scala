// Soft unions in inferred val and def types: kept when the join is transparent, widened to the join otherwise.
// expect: 26:29: error: type mismatch: found Shape, required Circle | Square
// expect: 30:21: error: type mismatch: found Named, required N1 | N2
// expect: 32:23: error: type mismatch: found Named & HasSize, required NS1 | NS2
// expect: 35:35: error: type mismatch: found Option[Int], required Some[Int] | None
// expect: 43:29: error: type mismatch: found Shape, required Circle | Square
// expect: 51:30: error: type mismatch: found Shape, required Circle | Square
// expect: 58:38: error: type mismatch: found Seq[Int], required List[Int] | Vector[Int]
trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class A(x: Int)
case class B(y: Int)
trait Named { def name: String }
case class N1() extends Named { def name = "n1" }
case class N2() extends Named { def name = "n2" }
trait HasSize { def size: Int }
case class NS1() extends Named, HasSize { def name = "ns1"; def size = 1 }
case class NS2() extends Named, HasSize { def name = "ns2"; def size = 2 }

object U:
  val c = true
  val v1 = if c then 1 else "a"
  val w1: Int | String = v1                 // ok: join Matchable is transparent, union kept
  val v2 = if c then Circle(1) else Square(2)
  val w2: Circle | Square = v2              // scalac: error, v2: Shape
  val v3 = if c then A(1) else B(2)
  val w3: A | B = v3                        // ok: join Product & Serializable transparent
  val v4 = if c then N1() else N2()
  val w4: N1 | N2 = v4                      // scalac: error, v4: Named
  val v5 = if c then NS1() else NS2()
  val w5: NS1 | NS2 = v5                    // scalac: error, v5: Named & HasSize
  val w5b: Named & HasSize = v5             // ok
  val v6 = if c then Some(1) else None
  val w6: Some[Int] | None.type = v6        // scalac: error, v6: Option[Int]
  val v7 = if c then 1 else 2.5
  val w7: Double = v7                       // ok: harmonised to Double
  val v8 = if c then Left(1) else Right("a")
  val w8: Either[Int, String] = v8          // ok
  def d1 = if c then 1 else "a"
  val x1: Int | String = d1                 // ok
  def d2 = if c then Circle(1) else Square(2)
  val x2: Circle | Square = d2              // scalac: error
  val v9 = c match
    case true => 1
    case false => "a"
  val w9: Int | String = v9                 // ok
  val v10 = c match
    case true => Circle(1)
    case false => Square(1)
  val w10: Circle | Square = v10            // scalac: error
  val hard: Circle | Square = if c then Circle(1) else Square(2)
  val v11 = hard
  val w11: Circle | Square = v11            // ok: hard union stays
  val v13 = if c then 1 else true
  val w13: Int | Boolean = v13              // ok (join AnyVal is transparent)
  val v14 = if c then List(1) else Vector(1)
  val w14: List[Int] | Vector[Int] = v14    // scalac: error, v14: Seq[Int]

@main def main(): Unit =
  println(U.v1)
  println(U.v3)
