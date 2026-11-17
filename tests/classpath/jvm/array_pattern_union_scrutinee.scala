// jars: scala-library
// A typed pattern with a wildcard or a type variable for an array's element (`case a: Array[?]`, `Array[t]`) over
// a scrutinee whose type is a union of array types takes every array of the union, as scalac's test of the
// written type does (`ScalaRunTime.isArray(x, 1)`): the wildcard is not the first member's element.
def kind(x: Array[Int] | Array[String] | String): String = x match
  case a: Array[?] => "array"
  case _ => "other"
def len(x: Array[Int] | Array[String]): Int = x match
  case a: Array[t] => a.length
def inSome(x: Option[Array[Int] | Array[String]]): String = x match
  case Some(a: Array[?]) => "array"
  case _ => "other"
def second(x: Array[Array[Int]] | Array[String] | Array[Int]): Int = x match
  case a: Array[Array[?]] => 1
  case a: Array[?] => 2
def generic[T](x: Array[Array[T]] | Array[T]): Int = x match
  case a: Array[Array[?]] => 1
  case a: Array[?] => 2
@main def run(): Unit =
  val ints: Array[Int] = Array(1, 2)
  val strs: Array[String] = Array("a")
  println("1 " + kind(ints) + " " + kind(strs) + " " + kind("s"))
  println("2 " + inSome(Some(ints)) + " " + inSome(Some(strs)))
  println("3 " + len(ints))
  println("4 " + (try len(strs).toString catch { case e: MatchError => "MatchError" }))
  println("5 " + second(Array(Array(1))) + " " + (try second(strs).toString catch { case e: MatchError => "MatchError" }) + " " + second(ints))
  println("6 " + generic[Int](Array(Array(1))) + " " + generic[Int](ints) + " " + (try generic[String](strs).toString catch { case e: MatchError => "MatchError" }))
