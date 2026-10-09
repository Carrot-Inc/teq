// Objects defined in a block: one instance per run of the block, made on first use, with
// the values around them captured, as scalac's lazy val holds them.
trait Greeter:
  def greet(name: String): String

sealed trait Cmd

object Main:
  def run(prefix: String): String =
    var hits = 0
    object Config:
      println(s"init $prefix")
      val separator = ":"
      def label(n: Int): String =
        hits += 1
        s"$prefix$separator$n"
      def twice(n: Int): String = Config.label(n) + separator + label(n)
    val before = hits
    val a = Config.label(1)
    val b = Config.twice(2)
    s"$before $a $b $hits ${Config.separator}"

  def unused(prefix: String): String =
    object Never:
      println(s"never $prefix")
    prefix

  def extractor(limit: Int): String =
    object Short:
      def unapply(s: String): Option[Int] = if s.length <= limit then Some(s.length) else None
    object Pair:
      def unapply(s: String): Option[(String, String)] = s.split("=") match
        case Array(k, v) => Some((k, v))
        case _ => None
    List("ab", "abcdef", "k=v").map {
      case Short(n) => s"short $n"
      case Pair(k, v) => s"$k -> $v"
      case _ => "other"
    }.mkString(", ")

  def caseObjects(n: Int): String =
    case object Marker
    case object Start extends Cmd
    case class Move(by: Int) extends Cmd
    val cmds: List[Cmd] = List(Start, Move(n), Start)
    val shown = cmds.map {
      case Start => "start"
      case Move(by) => s"move $by"
    }
    s"$Marker ${Marker == Marker} ${Marker.toString} ${shown.mkString(" ")} ${Start.productPrefix}"

  def implementing(name: String): Greeter =
    object Hello extends Greeter:
      def greet(who: String): String = s"$name greets $who"
    Hello

  def nested(depth: Int): String =
    object Outer:
      val d = depth
      def inner: String =
        object Inner:
          val d2 = d * 2
        s"${Inner.d2}"
    val f = () => Outer.inner + "/" + Outer.d
    f()

  def sameName(n: Int): Int =
    object A:
      def make = new A
      def y = n
    class A:
      val z = A.y * 2
    A.make.z

  def main(args: Array[String]): Unit =
    println(sameName(21))
    println(run("a"))
    println(run("b"))
    println(unused("u"))
    println(extractor(3))
    println(caseObjects(4))
    println(implementing("x").greet("y"))
    println(nested(5))
    val g = (k: Int) =>
      object Local:
        val v = k + 1
      Local.v
    println(g(1) + g(2))
