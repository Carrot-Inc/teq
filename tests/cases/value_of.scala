def name[S <: String & Singleton](using v: ValueOf[S]): String = v.value

case class Tagged(kind: "Point" = "Point", n: Int)

@main def run(): Unit =
  println(name["Point"])
  println(summon[ValueOf[1]].value + 1)
  println(summon[ValueOf[true]].value)
  println(Tagged(n = 2))
