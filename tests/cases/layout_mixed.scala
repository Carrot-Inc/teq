def render(title: String,
           body: Int => String): Int => String = { n =>
  val text = body(n)
  s"$title: $text"
}

def gate[A](a: A)(f: A => Int => String): Int => String = f(a)

def page(view: Int => String, actions: List[String]): String = view(1) + actions.mkString("[", ",", "]")

enum Level derives CanEqual: /* ordered by severity */
  case Low, High

def classify(n: Int, label: String): String = (n, label) match
  case (0, _) => val zero = "zero"; zero
  case (k, l)
    if k > 10 && l.nonEmpty => s"big $l"
  case (k, _) =>
    val small = "small"
    s"$small $k"

@main def main(): Unit =
  println(render("t", n => (n + 1).toString)(1))
  println(page(
    gate("x"): prefix =>
      n =>
        prefix + (
          n + 1
        ).toString,
    actions = List("a", "b")
  ))
  println(classify(0, "") + classify(11, "l") + classify(3, "z"))
  println(List(Level.Low, Level.High).map { level =>
    level match
    case Level.Low => 1
    case Level.High => 2
  })
