// The givens that provide an extension of the name are ranked as dotty's implicit search ranks
// them (`rank`, `compareAlternatives`): those in scope before the implicit scope's, the more
// deeply nested first, then the given whose owner derives from the other's, and where the givens
// tie, their extension methods compared (`disambiguate`): the more specific receiver, between
// two givens or a companion's extension and a given's; the first that provides one is not taken
// for being first. A given's overloads that do not take the arguments do not provide the
// extension (`bool`), and a narrower receiver found first is kept against two later ties (`HC`).
class R
class S
class T
trait Ops:
  extension (r: R) def pick: String
trait SOps:
  extension (s: S) def pick: String
trait TOps:
  extension (t: T) def pick: String
trait LowR:
  given low: Ops with
    extension (r: R) def pick: String = "R low"
object R extends LowR:
  given high: Ops with
    extension (r: R) def pick: String = "R high"
object S:
  given fromCompanion: SOps with
    extension (s: S) def pick: String = "S companion"
trait LowT:
  given lowT: TOps with
    extension (t: T) def pick: String = "T inherited"
object Outer extends LowT:
  given outerT: TOps with
    extension (t: T) def pick: String = "T outer"
  def nested(): String =
    given innerT: TOps = new TOps:
      extension (t: T) def pick: String = "T inner"
    (new T).pick
  def same(): String = (new T).pick
trait Bar1[A]:
  extension (x: A => A) def bar(y: A): Int
trait Bar2:
  extension (x: Int => 1) def bar(y: Int): Int
object Specific:
  given bla1: [X] => Bar1[X] = new Bar1[X]:
    extension (x: X => X) def bar(y: X): Int = 1
  given bla2: Bar2 = new Bar2:
    extension (x: Int => 1) def bar(y: Int): Int = 2
class U
trait AnyOps:
  extension (u: Any) def pick: String
trait UOps:
  extension (u: U) def pock: String
object U:
  extension (u: U) def pick: String = "U companion method"
  extension (u: Any) def pock: String = "U companion any"
  given a: AnyOps with
    extension (u: Any) def pick: String = "U given any"
  given b: UOps with
    extension (u: U) def pock: String = "U given"
class V
trait OA:
  extension (v: V)
    def pick(x: Int): String = "int"
    def pick(x: String): String = "string"
trait OB:
  extension (v: V) def pick(x: Boolean): String = "bool"
object Overloaded:
  given ob: OB = new OB {}
  given oa: OA = new OA {}
  def run(): String = (new V).pick(true)
class W
trait HA:
  extension (w: Any) def hpick: String = "HA"
trait HB:
  extension (w: Any) def hpick: String = "HB"
trait HC:
  extension (w: W) def hpick: String = "HC"
object HealFirst:
  given hc: HC = new HC {}
  given ha: HA = new HA {}
  given hb: HB = new HB {}
  def run(): String = (new W).hpick
object Main:
  given inScope: SOps with
    extension (s: S) def pick: String = "S in scope"
  def main(args: Array[String]): Unit =
    println((new R).pick)
    println((new S).pick)
    println(Outer.nested())
    println(Outer.same())
    import Specific.given
    val f: Int => 1 = x => 1
    println(f.bar(1))
    println((new U).pick)
    println((new U).pock)
    println(Overloaded.run())
    println(HealFirst.run())
