// A tuple past 22 elements seen as a `Product`: its prefix, `canEqual` and elements are its own
// class's members, as a call through `Product` dispatches.
@main def run(): Unit =
  val t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  val p: Product = t
  println(p.productPrefix)
  println(p.canEqual(t))
  println(p.canEqual((1, 2)))
  println(p.productIterator.size)
