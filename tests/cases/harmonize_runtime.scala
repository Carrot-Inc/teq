//> using platform js
// Run-time values after harmonisation: only Int literals convert, other branches keep their type.
object R:
  val c = true
  val i: Int = 3
  val b4 = if c then 1 else 'a'                  // scalac: Char (Int literal converted to Char); teq: Int?
  val b2 = if c then i else 2L                   // scalac: Int | Long holding an Int; teq: Long?
  val b2kind = b2 match
    case x: Int => "int"
    case x: Long => "long"
  val l3 = List(i, 4L)                           // scalac: List[Int | Long]
  val l3kind = l3.head match
    case x: Int => "int"
    case x: Long => "long"
  val bothLits = List(1, 2L)                     // scalac: List[Long]
  val bothLitsKind = if bothLits.head.isInstanceOf[Long] then "long" else "?"
  val mixed = List(1, "a")
  val mixedKind = mixed.head match
    case x: Int => "int"
    case _ => "other"
  val b6 = if c then i else 2.5                  // scalac: Int | Double holding an Int
  val b6kind = b6 match
    case x: Int => "int"
    case x: Double => "double"
  val d = if c then 1 else 2.0
  def show(x: Any): String = x.toString

@main def main(): Unit =
  println(R.b4.isInstanceOf[Char])
  println(R.show(R.b4))
  println(R.b2kind)
  println(R.l3kind)
  println(R.bothLitsKind)
  println(R.mixedKind)
  println(R.b6kind)
  println(R.d)
