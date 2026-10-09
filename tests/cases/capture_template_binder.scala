// A `@js` template that binds a name around an argument, `$0.map((x) => $1(x))`, would give
// that argument's reads of a local of the same name its own binding: the local is named apart.
@main def main(): Unit =
  val x = 10
  val arr = Array(1, 2, 3)
  println(arr.map(y => y + x).toList)
  println(arr.filter(y => y < x - 8).toList)
