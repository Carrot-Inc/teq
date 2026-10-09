// A direct call of an extension method names the extension's type parameters in its type
// arguments, as Scala reads P.nn[T](x); an array of unknown element type has a length.
object P:
  extension [T](x: T | Null) inline def nn: T = x.asInstanceOf[T]
  extension [T](x: Array[T]) def firstOr(d: T): T = if x.length > 0 then x(0) else d
def f(a: Array[?]): Int = a.length
@main def run(): Unit =
  val a: Array[Byte] | Null = Array[Byte](1, 2)
  println(P.nn[Array[Byte]](a).length)
  println(P.firstOr[Int](Array(7))(3))
  println(f(Array("x", "y")))
