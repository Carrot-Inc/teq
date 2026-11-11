package tov

def top(using s: String)(x: Int): String = "top-using:" + s + x
def top(s: String): Int => String = x => "top-plain:" + s + x

object Lib:
  def topLiteral: String = top(using "T")(6)
  def topPlain: String = top("Q")(7)
