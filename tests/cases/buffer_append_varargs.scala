// The deprecated `append` of several elements, which a library still calls, beside the one of
// one element; on an array buffer and a list buffer.
import scala.collection.mutable

@main def run =
  val b = mutable.ArrayBuffer(1)
  b.append(2, 3)
  b.append(4)
  println(b)
  val l = mutable.ListBuffer("a")
  l.append("b", "c")
  l.append("d")
  println(l)
