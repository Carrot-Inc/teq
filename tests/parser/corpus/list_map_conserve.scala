// `List.mapConserve`: the list itself when every result is its element, else the results with
// the elements after the last change shared; every element mapped once, in order.
@main def run =
  val xs: List[String] = List("a", "b", "c", "d")
  println(xs.mapConserve(x => x) eq xs)
  var seen = List.empty[String]
  val changed = xs.mapConserve { x =>
    seen = x :: seen
    if x == "b" then "B" else x
  }
  println(changed)
  println(seen.reverse)
  println(changed.tail.tail eq xs.tail.tail)
  println(changed.tail ne xs.tail)
  println(List.empty[String].mapConserve(_ + "!"))
  println(xs.mapConserve(_.toUpperCase))
