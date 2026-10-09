// jars: scala-library
// std: lean scala-library
// An operand of a concatenation takes the slots of what is passed: a value class over a `Long`
// or a `Double` is passed as its box, one slot, so a hundred of them are one call, which
// renders none of them before the last operand is evaluated.
object WideProbe:
  var rendered = 0

  final class W(val n: Long) extends AnyVal:
    override def toString: String =
      rendered += 1
      "v"

  final class D(val d: Double) extends AnyVal:
    override def toString: String =
      rendered += 1
      "d"

  def longs(): Unit =
    val w = new W(1L)
    val s =
      "x" +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      { println("before=" + rendered); "" }
    println("after=" + rendered + " " + s.length)

  def doubles(): Unit =
    rendered = 0
    val d = new D(1.5)
    val w = 2L
    val s =
      "x" +
      d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d +
      d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d +
      d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d +
      d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d +
      d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d + d +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w + w +
      w + w + w + w + w + w + w + w + w +
      { println("before=" + rendered); "" } +
      w +
      { println("then=" + rendered); "" }
    println("after=" + rendered + " " + s.length)

@main def run(): Unit =
  WideProbe.longs()
  WideProbe.doubles()
