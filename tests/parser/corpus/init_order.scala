trait Codec[T]:
  def name: String
  def enc(t: T): String
object Codec:
  given Codec[Int] with
    def name: String = "int"
    def enc(t: Int): String = "#" + t
  given Codec[String] with
    def name: String = "str"
    def enc(t: String): String = "\"" + t + "\""

enum Planet(val mass: Double, val radius: Double):
  val gravity: Double = 6.67e-11 * mass / (radius * radius)
  val label: String = s"$productPrefix($ordinal) g=" + gravity.toInt
  case Mercury extends Planet(3.303e+23, 2.4397e6)
  case Earth extends Planet(radius = 6.37814e6, mass = 5.976e+24)
  def weight(other: Double): Double = other * gravity

enum Field[T: Codec](val name: String, val default: T):
  val codecName: String = summon[Codec[T]].name
  val encodedDefault: String = summon[Codec[T]].enc(default)
  val summary: String = s"$name:$codecName=$encodedDefault"
  case Age extends Field[Int]("age", 18)
  case Nick extends Field[String]("nick", "anon")
  case Other[T: Codec](n: String, d: T) extends Field[T](n + "!", d)

final case class Money[T: Codec](amount: T, currency: String):
  val encoded: String = summon[Codec[T]].enc(amount) + " " + currency
  val twice: String = encoded + encoded
  println("init " + twice)

final case class Range3(lo: Int, hi: Int):
  val size: Int = hi - lo
  val mid: Int = lo + size / 2
  var hits: Int = mid * 2

object Registry:
  println("registry start")
  val all: List[String] = List[Field[?]](Field.Age, Field.Nick).map(_.summary)
  val first: String = all.head
  println("registry done")

@main def main(): Unit =
  println("main start")
  println(Planet.Earth.label)
  println(Planet.Mercury.label)
  println(Planet.Earth.weight(10).toInt)
  println(Field.Age.summary)
  println(Field.Nick.summary)
  println(Field.Other("x", 3).summary)
  println(Field.Other("x", 3).productPrefix)
  println(Registry.first)
  val m = Money(5, "USD")
  println(m.twice)
  println(m.copy(amount = 7).twice)
  println(m)
  println(m == Money(5, "USD"))
  val r = Range3(2, 10)
  println((r.size, r.mid, r.hits))
  println(Planet.values.toList.map(_.gravity.toInt))
  println(Planet.valueOf("Earth").ordinal)
  println(Planet.fromOrdinal(0))
