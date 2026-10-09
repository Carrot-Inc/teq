import java.util.Objects.deepEquals

// `java.util.Objects.deepEquals`: arrays element by element, nested arrays deeply, anything else
// by `equals`, a boxed `1` unequal to a boxed `1L`, floating points by their bits.
@main def run(): Unit =
  println(deepEquals(Array[Byte](1, 2), Array[Byte](1, 2)))
  println(deepEquals(Array[Byte](1, 2), Array[Byte](1, 3)))
  println(deepEquals(Array(Array("a"), Array("b")), Array(Array("a"), Array("b"))))
  println(deepEquals(Array("a"), Array("a", "b")))
  println(deepEquals("x", "x"))
  println(deepEquals(null, null))
  println(deepEquals(Array(1), null))
  println(deepEquals(1, 1L))
  println(deepEquals(Array[Any](1), Array[Any](1L)))
  println(deepEquals(2.0, 2.0))
  println(deepEquals(Array[AnyRef](java.lang.Double.valueOf(-0.0)), Array[AnyRef](java.lang.Double.valueOf(0.0))))
  println(deepEquals(Array[AnyRef](java.lang.Double.valueOf(Double.NaN)), Array[AnyRef](java.lang.Double.valueOf(Double.NaN))))
  println(deepEquals(-0.0f, 0.0f))
  println(deepEquals(Float.NaN, Float.NaN))
