// A lambda, placeholder lambda or eta-expansion takes its parameter types from the one function
// member of a union expected type.
final case class Props(
  onChange: (String => Unit) | Unit = (),
  parse: ((String, Int) => String) | Unit = (),
  render: (Int => String) | String = "static",
)
def handler(s: String): Unit = println("h " + s)
def run(p: Props): Unit =
  p.onChange match
    case _: Unit => println("none")
    case f =>
      val g = f.asInstanceOf[String => Unit]
      g("abc")
  p.parse match
    case _: Unit => println("no parse")
    case f =>
      val g = f.asInstanceOf[(String, Int) => String]
      println(g("ab", 2))
  p.render match
    case s: String => println(s)
    case f =>
      val g = f.asInstanceOf[Int => String]
      println(g(3))

@main def main(): Unit =
  run(Props(onChange = s => println(s.length)))
  run(Props(onChange = println(_), parse = (s, i) => s * i))
  run(Props(onChange = handler))
  run(Props(onChange = _.length, render = i => "n" + i))
  val g: ((Int, Int) => Int) | Unit = _ + _
  g match
    case _: Unit => println("unit")
    case f =>
      val h = f.asInstanceOf[(Int, Int) => Int]
      println(h(2, 3))
  val pairs: ((Int, Int) => String) | Unit = (a, b) => s"$a-$b"
  pairs match
    case _: Unit => println("unit")
    case f =>
      val h = f.asInstanceOf[(Int, Int) => String]
      println(h(1, 2))
