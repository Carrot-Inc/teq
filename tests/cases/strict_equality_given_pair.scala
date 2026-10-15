// Under strict equality, a given of the compared pair itself (indigo's `given CanEqual[Option[Camera], Option[Camera]]`
// in `Camera`'s companion) allows a comparison the standard instances refuse, as `Implicits.checkCanEqual` searches the
// pair's implicit scope; and `scala.util.Random`'s `between` of longs, doubles and floats (indigo's `Context.Dice`).
import scala.language.strictEquality

sealed trait Camera
object Camera:
  final case class Fixed(x: Int) extends Camera
  given CanEqual[Fixed, Fixed] = CanEqual.derived
  given CanEqual[Option[Camera], Option[Camera]] = CanEqual.derived

@main def run(): Unit =
  val a: Option[Camera] = Some(Camera.Fixed(1))
  val b: Option[Camera] = Some(Camera.Fixed(1))
  val c: Option[Camera] = Some(Camera.Fixed(2))
  println(a == b)
  println(a == c)
  println(Camera.Fixed(3) == Camera.Fixed(3))
  val r = new scala.util.Random(42)
  println(r.between(1L, 100L))
  println(r.between(-5L, 5L))
  println((r.between(1.0, 2.0) * 1000).toInt)
  println((r.between(1.0f, 2.0f) * 1000).toInt)
  println(r.between(3, 9))
