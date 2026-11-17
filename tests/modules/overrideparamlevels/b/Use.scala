// The classes of the library extended here: a body under its parameter class, a parameter under its body class,
// anonymous classes and classes local to a method; each line is scalac's.
import opl.*
import scala.annotation.nowarn
@nowarn class U1 extends C1("c-arg") { override val v: String = "u1-body" }
@nowarn class U2(override val v: String) extends C2
@nowarn class U3(override val t: String) extends LB
@nowarn @main def run(): Unit =
  def r(p: P): String = p.v + "/" + p.read
  println("param-body-param " + r(new A1("a")) + " " + r(new B1) + " " + r(new C1("c")))
  println("body-param-body " + r(new A2) + " " + r(new B2("b")) + " " + r(new C2))
  println("siblings " + r(new S1("s1")) + " " + r(new S3) + " " + r(new S4) + " " + r(new P))
  println("trait-param-body " + new K("k").t + "/" + new K("k").readT + " " + new L().t + "/" + new L().readT)
  println("trait-body-param " + new M().t + "/" + new M().readT + " " + new N("n").t + "/" + new N("n").readT)
  println("trait-under-class " + new KB("kb").readT + " " + new LB().t + "/" + new LB().readT + " " + new U3("u3").readT)
  println("abstract " + new AV1("av1").read + " " + new AV2().v + "/" + new AV2().read + " " + new AV3("av3").v + "/" + new AV3("av3").read)
  println("lazy " + new LZ().seen + " " + new LZ1("x").seen + " " + new LZ2().seen + " " + new LZ2().lz + " " + new LZ1("x").lz)
  println("generic " + new G1(Some("g1")).readO + " " + new G2().readO + " " + new G3(Some("g3")).readO + " " + new GP[Int].readO)
  println("object " + r(Oc))
  println("anonymous " + r(new B1 { override val v: String = "anon-body" }) + " " + r(new A1("x") {}) + " " + r(new C1("y") { def extra: Int = 1 }))
  println("user-chains " + r(new U1) + " " + r(new U2("u2")))
  println("param-over-param " + new QU().q + "/" + new QU().readQ + " " + new QC("c").q + "/" + new QC("c").readQ + " " + new Q("q").q)
  println("param-copy " + new Cb("child").body + " " + new Cb("child").readP + " " + new Pb("p").body)
  println("own-read " + new RA("a").got + " " + new RB().got + "/" + new RB().read + " " + new RC("c").got + "/" + new RC("c").read)
  println("trait-own-read " + new K5("k").got + " " + new L5().got + "/" + new L5().t)
  def local(): String =
    class Loc(override val v: String) extends P
    @nowarn class LocB extends Loc("loc-arg") { override val v: String = "locb-body" }
    new Loc("user-loc").read + " " + new LocB().read
  println("local " + local() + " " + Lib.local())
