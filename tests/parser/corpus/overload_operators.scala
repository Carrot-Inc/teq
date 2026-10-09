//> using scala 3.8.4
case class Money(cents: Int):
  def +(other: Money): Money = Money(cents + other.cents)
  def +(extra: Int): Money = Money(cents + extra)
  def +(a: Money, b: Money): Money = this + a + b
  def *(factor: Int): Money = Money(cents * factor)
  def *(factor: Double): Money = Money((cents * factor).toInt)
  def unary_- : Money = Money(-cents)
  def <(other: Money): Boolean = cents < other.cents
  def <(limit: Int): Boolean = cents < limit

class Log:
  private var lines = List.empty[String]
  def +=(line: String): Log = { lines = lines :+ line; this }
  def +=(code: Int): Log = { lines = lines :+ ("#" + code); this }
  def apply(i: Int): String = lines(i)
  def apply(prefix: String): List[String] = lines.filter(_.startsWith(prefix))
  def update(i: Int, v: String): Unit = lines = lines.updated(i, v)
  def update(prefix: String, v: String): Unit = lines = lines.map(l => if l.startsWith(prefix) then v else l)

@main def run(): Unit =
  val m = Money(100)
  println(m + Money(5))
  println(m + 7)
  println(m + (Money(1), Money(2)))
  println(m * 2)
  println(m * 1.5)
  println(-m)
  println(m < Money(200))
  println(m < 50)
  val log = Log()
  log += "start"
  log += 42
  println(log(0) + " " + log(1))
  println(log("#"))
  log(0) = "begin"
  log("#") = "code"
  println(log(0) + " " + log(1))
