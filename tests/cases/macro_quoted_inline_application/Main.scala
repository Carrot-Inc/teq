case class P(name: String, age: Int)
@main def run(): Unit =
  println(G[P](_.name))
  println(G[P](_.age))
