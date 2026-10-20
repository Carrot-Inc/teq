// lila: stringAnyValHandler(_.toString, FideTC.valueOf) with chess.FideTC a Scala 3 enum.
// The enum companion's synthesized valueOf / fromOrdinal are ordinary methods, so they can
// be used as function values (eta-expansion), not only applied.
enum FideTC:
  case standard, rapid, blitz

def handler[A](to: A => String, from: String => A): String => A = s => from(to(from(s)))

@main def run() =
  val read = handler[FideTC](_.toString, FideTC.valueOf)
  println(read("rapid"))
  println(List(0, 2).map(FideTC.fromOrdinal))
