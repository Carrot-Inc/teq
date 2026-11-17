// A val a class takes as a constructor parameter keeps the parameter's value whatever lies above the class (a body's
// definition, a trait's val, an abstract val, another parameter) and gives way to a body's redefinition below it;
// the levels of a hierarchy mixed: a parameter over a body over a parameter, a body over a parameter over a body, a
// trait's val under a class and under a class with another parent, lazy vals, a generic parent, an object, a class
// whose body reads its own parameter, classes local to a method. scalac warns of a body's redefinition of a val
// parameter (silenced: the values are the test).
package opl
import scala.annotation.nowarn
class P { val v: String = "p"; def read: String = v }
class A1(override val v: String) extends P
@nowarn class B1 extends A1("a-arg") { override val v: String = "b-body" }
@nowarn class C1(override val v: String) extends B1
class A2 extends P { override val v: String = "a-body" }
class B2(override val v: String) extends A2
@nowarn class C2 extends B2("b-arg") { override val v: String = "c-body" }
class S1(override val v: String) extends P
class S3 extends P { override val v: String = "s3-body" }
class S4 extends P
trait T { val t: String = "trait"; def readT: String = t }
class K(override val t: String) extends T
@nowarn class L extends K("k-arg") { override val t: String = "l-body" }
class M extends T { override val t: String = "m-body" }
class N(override val t: String) extends M
class Base
trait TB { val t: String = "tb"; def readT: String = t }
class KB(override val t: String) extends Base with TB
@nowarn class LB extends KB("kb-arg") { override val t: String = "lb-body" }
abstract class AV { val v: String; def read: String = v }
class AV1(val v: String) extends AV
@nowarn class AV2 extends AV1("av1-arg") { override val v: String = "av2-body" }
@nowarn class AV3(override val v: String) extends AV2
class LZ { val v: String = "p"; lazy val seen: String = "lazy " + v; lazy val lz: String = "p-lazy" }
class LZ1(override val v: String) extends LZ
@nowarn class LZ2 extends LZ1("lz1-arg") { override val v: String = "lz2-body"; override lazy val lz: String = "lz2-lazy" }
class GP[X] { val o: Option[X] = None; def readO: Option[X] = o }
class G1(override val o: Option[String]) extends GP[String]
@nowarn class G2 extends G1(Some("g1-arg")) { override val o: Option[String] = Some("g2-body") }
@nowarn class G3(override val o: Option[String]) extends G2
object Oc extends C1("oc-arg")
class Q(val q: String) { def readQ: String = q }
@nowarn class QU extends Q("q") { override val q: String = "qu-body" }
@nowarn class QC(override val q: String) extends QU
class Pb(val p: String) { val body: String = "saw " + p; def readP: String = p }
class Cb(override val p: String) extends Pb("pb-arg")
class R { val v: String = "r"; def read: String = v }
class RA(override val v: String) extends R { val got: String = "got " + v }
@nowarn class RB extends RA("ra-arg") { override val v: String = "rb-body" }
@nowarn class RC(override val v: String) extends RB
trait T5 { val t: String = "trait" }
class K5(override val t: String) extends T5 { val got: String = "got " + t }
@nowarn class L5 extends K5("k5-arg") { override val t: String = "l5-body" }
object Lib:
  def local(): String =
    class Loc(override val v: String) extends P
    @nowarn class LocB extends Loc("loc-arg") { override val v: String = "locb-body" }
    new Loc("lib-loc").read + " " + new LocB().read
