// Unrelated branch and element types join to their union, related ones to their common base type.
sealed trait Node
final case class Tag(name: String) extends Node
case object EmptyNode extends Node
final case class Wrap(n: Node)
enum Field[T]:
  case Age extends Field[Int]
  case Nick extends Field[String]
def show(x: Tag | Int | String): String = x match
  case t: Tag => "tag " + t.name
  case i: Int => "int " + i
  case s: String => "str " + s

@main def main(): Unit =
  val c = "x".length == 1
  val n = if c then Tag("a") else EmptyNode
  println(Wrap(n))
  val v = if c then 1 else "a"
  println(show(v))
  val w = List(1, "a")
  w.foreach(x => println(show(x)))
  val fs = List(Field.Age, Field.Nick)
  println(fs)
  val xs = List(Tag("q"), "a", 2)
  xs.foreach(x => println(show(x)))
  val m = Map("a" -> 1, "b" -> "x")
  m.values.foreach(x => println(show(x)))
  val ys = xs.map(x => if c then x else "z")
  ys.foreach(x => println(show(x)))
  val f = (b: Boolean) => if b then Tag("z") else 1
  println(show(f(true)))
  val o = Option(1).map(i => if i > 0 then "pos" else i)
  println(o.map(show))
  val s = Set(1, "a")
  println(s.size)
  val matched = c match
    case true => Tag("m")
    case false => "no"
  println(show(matched))
  val opt = if c then Some(1) else None
  println(opt)
  val ints = if c then 1 else 2L
  println(ints)
  val v2 = Vector(Some(1), None, Some(2))
  println(v2)
