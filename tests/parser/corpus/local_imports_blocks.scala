enum Mode:
  case Fast, Slow

object Tools:
  def helper(n: Int): Int = n + 100
  val label: String = "tools"
  extension (s: String)
    def twice: String = s + s
  given Ordering[Mode] = Ordering.by(_.ordinal)

object Other:
  val label: String = "other"

def inCase(o: Option[Mode]): String = o match
  case Some(m) =>
    import Mode.*
    m match
      case Fast => "fast"
      case Slow => "slow"
  case None =>
    import Tools.*
    label.twice

def inIf(b: Boolean): Int =
  if b then
    import Tools.helper
    helper(1)
  else
    import Tools.{helper as h}
    h(2)

def inBraces(): Int = {
  import Tools.*
  val f = (n: Int) => {
    import Other.*
    label.length + helper(n)
  }
  f(1) + label.length
}

def inFor(xs: List[Int]): List[Int] =
  for
    x <- xs
  yield
    import Tools.*
    helper(x)

def localDefs(): String =
  import Tools.*
  def inner(n: Int): Int = helper(n) * 2
  val g = () => label.twice
  s"${inner(1)} ${g()}"

def sorted(): List[Mode] =
  import Tools.given
  List(Mode.Slow, Mode.Fast).sorted

class Holder:
  import Tools.*
  val v: String = label.twice
  lazy val w: Int = helper(v.length)
  def m(n: Int) =
    import Other.label
    label + helper(n)

def shadowOuterImport(): String =
  import Tools.*
  val a = label
  val b =
    import Other.*
    label
  a + "/" + b + "/" + label

@main def main(): Unit =
  println(inCase(Some(Mode.Fast)))
  println(inCase(None))
  println(inIf(true) + inIf(false))
  println(inBraces())
  println(inFor(List(1, 2)))
  println(localDefs())
  println(sorted())
  val h = Holder()
  println(h.v + h.w + h.m(1))
  println(shadowOuterImport())
