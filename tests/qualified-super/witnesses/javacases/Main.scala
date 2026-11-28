class JB extends JavaBase { class I { def f = JB.super.m() } }
class JD extends JavaDefault { class I { def f = JD.super.m() } }
trait S { def m(): String = "S" }
class SB extends S { class I { def f = SB.super.m() } }
class G[X] { def m(x: X): X = x }
class SG extends G[String] { override def m(x: String): String = x }
class BG extends SG { class I { def f = BG.super.m("bridge") } }
@main def run(): Unit = { val b = new JB; val d = new JD; val s = new SB; val g = new BG; println((new b.I).f); println((new d.I).f); println((new s.I).f); println((new g.I).f) }
