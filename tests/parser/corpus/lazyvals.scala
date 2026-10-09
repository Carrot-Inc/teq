class Config(name: String):
  lazy val loaded: String =
    println(s"loading $name")
    name.toUpperCase
  val eager: Int = 1

object Holder:
  lazy val big: List[Int] =
    println("building")
    List(1, 2, 3)

@main def run(): Unit =
  val c = Config("app")
  println("created")
  println(c.loaded)
  println(c.loaded)
  lazy val local =
    println("local init")
    42
  println("before local")
  println(local + local)
  println(Holder.big.sum)
  println(Holder.big.length)
