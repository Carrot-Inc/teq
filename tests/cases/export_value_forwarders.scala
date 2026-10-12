// Export clauses whose qualifier is a value make forwarders, as dotty's Namer.exportForwarders:
// final members of the class calling the member on the path, with its signature as the path sees
// it, a default left out given by the member, an abstract member of a parent implemented, a
// wildcard leaving out what the clause names otherwise.
class Info(val ply: Int, val color: String):
  def describe(prefix: String, n: Int = 1): String = s"$prefix $ply $color $n"
  def twice[A](a: A): List[A] = List(a, a)
  def pick(n: Int): String = s"int $n"
  def pick(s: String): String = s"string $s"
  lazy val lz = ply * 10
  given label: String = s"info-$ply"

trait HasInfo:
  val info: Info
  export info.{ ply, color, describe as desc, twice, lz }

case class Advice(info: Info, text: String) extends HasInfo

class Game(val chess: Info):
  def color: String = "own " + chess.color
  export chess.{ lz as _, label as _, color as _, * }

class Box[A](val value: A):
  def get: A = value
  def map[B](f: A => B): Box[B] = Box(f(value))

class Holder(b: Box[Int]):
  export b.{ get, map }

trait Ext:
  def baseUrl: String
class Net(val baseUrl: String, val port: Int)
class Env(val net: Net)
class Ctrl(env: Env) extends Ext:
  export env.net.baseUrl

object Config:
  val default = Info(9, "red")
  export default.{ ply as defaultPly }

class Givens(info: Info):
  export info.given
  def show(using s: String) = s

@main def run(): Unit =
  val a = Advice(Info(3, "white"), "t")
  println((a.ply, a.color, a.desc("p"), a.desc("p", 2), a.twice("x"), a.lz))
  val g = Game(Info(5, "black"))
  println((g.ply, g.color, g.describe("q"), g.pick(4), g.pick("s")))
  val h = Holder(Box(20))
  println((h.get + 1, h.map(_ * 2).get))
  val e: Ext = Ctrl(Env(Net("https://x", 80)))
  println(e.baseUrl)
  println(Config.defaultPly)
  val gv = Givens(Info(7, "blue"))
  import gv.given
  println(gv.show)
