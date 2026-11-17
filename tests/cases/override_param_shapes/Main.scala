package c
import scala.annotation.nowarn
class P extends b.P("b.P-arg")
class K(override val v: String) extends a.P
// a plain parameter of the val's name
class M(override val v: String) extends a.P
class Plain(v: String) extends a.P { def show: String = v }
class PlainBody(v: String) extends a.P { val got: String = "got " + v }
class PlainUnread(v: String) extends a.P
// an overloaded name beside the val
class PO { val v: String = "po"; def v(i: Int): String = "po" + i; def read: String = v }
class AO(override val v: String) extends PO
class PC { val v: String = "pc"; def read: String = v }
class AC(override val v: String) extends PC { def v(i: Int): String = "ac" + i }
trait TE { val t: String = "te"; def t(i: Int): String = "te" + i; def readT: String = t }
class AE(override val t: String) extends TE
enum En(val tag: String):
  def tag(i: Int): String = tag * i
  def readTag: String = tag
  case E1 extends En("e1")
  case E2(override val tag: String) extends En("e2-arg")
// a trait's parameter
trait T { val t: String = "trait"; val tSaw: String = "T-saw:" + t }
trait TQ(override val t: String) extends T
class X1 extends TQ("q1")
@nowarn class X3 extends TQ("q3") { override val t: String = "x3-body" }
// names with a dollar sign
class D1 { val v: String = "d1"; val r$v$D1: String = "d1-r"; def read: String = v + "/" + r$v$D1 }
class D2(override val r$v$D1: String) extends D1
class D3(override val v: String) extends D1
class x$P { val v: String = "x$P.v"; def readV: String = v }
class PD extends x$P { val v$b: String = "PD.v$b"; def readVb: String = v$b }
class CD(override val v$b: String) extends PD
class DD(override val v: String) extends x$P
@main def run(): Unit =
  println("packages " + new b.P("k").read + "/" + new b.P("k").v + " " + new P().read + " " + new K("k2").read + " " + new a.P().read)
  println("packages-body " + new b.Q().read + " " + new b.A().read + " " + new a.A("a").read)
  println("plain " + new Plain("x").read + "," + new Plain("x").show + " " + new PlainBody("y").read + "," + new PlainBody("y").got + " " + new PlainUnread("z").read + " " + new M("m").read)
  println("overloaded " + new AO("ao").read + "/" + new AO("ao").v + " " + new PO().read + " " + new AC("ac").read + "/" + new AC("ac").v(1) + " " + new AE("ae").readT + "/" + new AE("ae").t(2))
  println("overloaded-enum " + En.E1.readTag + " " + En.E2("e2").tag)
  println("trait-parameter " + new X1().t + " " + new X3().tSaw + " " + new X3().t)
  println("dollar " + new D2("d2").read + " " + new D3("d3").read + " " + new D1().read + " " + new CD("c").readV + "," + new CD("c").readVb + " " + new DD("d").readV + " " + new PD().readV)
