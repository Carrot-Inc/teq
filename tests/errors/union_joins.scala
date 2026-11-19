// expect: 18:21: error: Found:    Tag | Int⏎Required: Nothing
// expect: 20:21: error: Found:    List[Tag | String]⏎Required: Nothing
// expect: 22:21: error: Found:    List[Field[? >: Int & String <: (Int | String)]]⏎Required: Nothing
// expect: 24:21: error: Found:    Option[Int]⏎Required: Nothing
// expect: 26:21: error: Found:    Set[Int | String]⏎Required: Nothing
// expect: 28:21: error: Found:    Boolean => Tag | Int⏎Required: Nothing
// expect: 30:21: error: Found:    Double⏎Required: Nothing
// expect: 34:21: error: Found:    Map[Int, String]⏎Required: Nothing

final case class Tag(name: String)
enum Field[T]:
  case Age extends Field[Int]
  case Nick extends Field[String]

@main def main(): Unit =
  val c = "x".length == 1
  val a = if c then Tag("a") else 1
  val na: Nothing = a
  val b = List(Tag("a"), "s")
  val nb: Nothing = b
  val f = List(Field.Age, Field.Nick)
  val nf: Nothing = f
  val o = if c then Some(1) else None
  val no: Nothing = o
  val s = Set(1, "a")
  val ns: Nothing = s
  val g = (b: Boolean) => if b then Tag("z") else 1
  val ng: Nothing = g
  val d = if c then 1 else 2.5
  val nd: Nothing = d
  val m = c match
    case true => Map(1 -> "a")
    case false => Map.empty
  val nm: Nothing = m
