// java.lang.Byte's and Short's MAX_VALUE and MIN_VALUE are constants, as their class file's ConstantValue makes them,
// and so are scala.Byte's and Short's MaxValue and MinValue, scala-library's `final val`s of them: inline vals, an inline
// if and stable identifier patterns read them. scalac prints the lines of the .expected file.
object D:
  inline val b = Byte.MaxValue
  inline val s = Short.MinValue
  inline def f: Byte = { inline val x = Byte.MinValue; x }
  inline val j = java.lang.Short.MAX_VALUE
  inline def big: String = inline if java.lang.Short.MAX_VALUE > 100 then "short max over 100" else "not"
def kind(x: Int): String = x match
  case java.lang.Byte.MAX_VALUE => "max"
  case java.lang.Byte.MIN_VALUE => "min"
  case _ => "other"
@main def run(): Unit =
  println(List(D.b, D.s, D.f, D.j))
  println(D.big)
  println(List(kind(127), kind(-128), kind(0)))
  val read: Byte = java.lang.Byte.MAX_VALUE
  println((read, Byte.MinValue, Short.MaxValue))
