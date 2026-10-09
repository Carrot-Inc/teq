// `values` of a generic enum is an `Array[E[?]]`: its elements' members read through the
// wildcard, and the array converts to a collection for an extension (scalac: "a,b", "b", "2").
trait ST[T]
given ST[Int] = new ST[Int] {}
given ST[String] = new ST[String] {}
enum Setting[T: ST](val key: String):
  case A extends Setting[Int]("a")
  case B extends Setting[String]("b")
extension [A](as: IterableOnce[A]) def keys(f: A => String): String = as.iterator.map(f).mkString(",")
@main def main(): Unit =
  val vs = Setting.values
  println(vs.sortBy(_.key).keys(_.key))
  println(Setting.valueOf("B").key)
  println(vs.length)
