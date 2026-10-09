@main def run(): Unit =
  val b = pl.PlBox.of(1)
  println(pl.Places.direct(b) + pl.Places.nested(b) + pl.Places.one + pl.Places.two + pl.Places.applied(b) + pl.Places.operator(b) + pl.Places.fine(b))
