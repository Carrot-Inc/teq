import X.given

object U:
  inline def fetch: Int = compiletime.summonInline[Box].v
  val boxed = fetch
  inline def twice(n: Int): Box = new Box { def v = n * 2 }
  val made = twice(4)

@main def run(): Unit =
  println(U.boxed)
  println(U.made.v)
