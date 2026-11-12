// An enum companion's `valueOf` and `fromOrdinal` are methods of the companion (`DesugarEnums.enumScaffolding`,
// DesugarEnums.scala 136–138; `fromOrdinalMeth`, 278–280), so a reference to one without arguments is the function of
// its argument, as any method's is: under an expected function type, as an argument, with no expected type, as the
// argument of `andThen`, through a val that holds the companion; for an enum whose cases extend it, one with a
// written companion, a generic one, one with a class case (`fromOrdinal` alone), one nested in an object and one
// nested in a class. scalac 3.8.4 prints the .expected.
enum Tone:
  case Low, High

enum Metric:
  case Grams extends Metric
  case Each extends Metric

enum Slot:
  case Left, Right
object Slot:
  val first: Slot = Left

enum Sign[+T]:
  case Neg extends Sign[Int]
  case Pos extends Sign[Int]

enum Shape:
  case Dot
  case Circle(r: Int)

object Outer:
  enum In:
    case A, B

class Host:
  enum In:
    case A, B

def use[T](f: String => T): T = f("High")

@main def main(): Unit =
  val t: String => Tone = Tone.valueOf
  println(t("Low"))
  println(use(Tone.valueOf))
  println(List("Low", "High").map(Tone.valueOf))
  val o: Int => Tone = Tone.fromOrdinal
  println(o(1))
  println(List(0, 1).map(Tone.fromOrdinal))
  val untyped = Tone.valueOf
  println(untyped("High"))
  val trimmed = ((s: String) => s.trim).andThen(Tone.valueOf)
  println(trimmed(" Low "))
  val companion = Tone
  println(List("High").map(companion.valueOf))
  val m: String => Metric = Metric.valueOf
  println(m("Each"))
  val s: String => Slot = Slot.valueOf
  println(s("Right"))
  println(List("Pos").map(Sign.valueOf))
  println(List(0).map(Shape.fromOrdinal))
  println(List("B").map(Outer.In.valueOf))
  val host = new Host
  println(List("A").map(host.In.valueOf))
  println(List(1).map(host.In.fromOrdinal))
