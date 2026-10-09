package ea

object Syntax:
  extension (s: String)
    def shout: String = s.toUpperCase + "!"
    def repeat(n: Int): String = s * n
  extension [A](xs: List[A])
    def second: A = xs(1)
  extension (n: Int) def +:(s: String): String = s + n

extension (d: Double) def half: Double = d / 2
