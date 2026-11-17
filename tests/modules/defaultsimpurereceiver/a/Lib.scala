package lib
object Log { def t[A](s: String, a: A): A = { println(s); a } }
import Log.t
final class Box(var n: Int) { def +=(r: Int): Unit = { println("Box.+= " + r); n = n + r }; def bump(r: Int): Unit = { println("Box.bump " + r); n = n + r } }
class AD { val s = new Box(1); def apply(i: Int = 0, j: Int = 1): Box = { println("apply " + i + " " + j); s } }
object L {
  val ad = new AD
  def run(): Unit = { t("ad", ad)() += 2; println(ad.s.n) }
}
