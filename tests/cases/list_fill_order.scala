// `List.fill` computes its element in the list's order, as scala-library's `View.Fill` does: an
// element that reads a stream reads it front to back (Scala.js's test bridge deserializes every
// list of its messages so).
@main def run(): Unit =
  var k = 0
  println(List.fill(4) { k += 1; k })
  val it = Iterator("a", "b", "c")
  println(List.fill(3)(it.next()))
  println(Vector.fill(3) { k += 1; k })
  println(Array.fill(2) { k += 1; k }.toList)
