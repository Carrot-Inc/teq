// A type parameter conforms to a refinement through the term member that a refinement in its upper bound
// declares, and so does a higher-kinded parameter applied to arguments: the bound written on the parameter, reached
// through another parameter, an intersection or an alias; a `def`, a `val` and a member with parameters; the
// parameter as a type argument of a class whose own parameter has such a bound; and the parameter inside another
// one's bound or inside the value's type: a side of an intersection, a refinement's parent, an alias's argument.
// scalac 3.8.4 prints the same lines.
import scala.reflect.Selectable.reflectiveSelectable

class Knot { def self: Int = 1; def more: Int = 2; val size: Int = 3; def add(n: Int): Int = n + 4; def text: String = "t" }
trait Named { def name: String = "n" }
class Tied extends Knot with Named
class Loop[X](x: X) { def self: X = x; def fixed: Int = 5 }
type Selfish = AnyRef { def self: Int }

def keep[T <: AnyRef { def self: Int }](t: T): AnyRef { def self: Int } = t
def chain[T <: AnyRef { def self: Int }, U <: T, V <: U](v: V): AnyRef { def self: Int } = v
def both[T <: Named & { def self: Int }](t: T): AnyRef { def self: Int } = t
def left[T <: AnyRef { def self: Int }](t: T & Named): AnyRef { def self: Int } = t
def aliased[T <: Selfish](t: T): Selfish = t
def applied[F[X] <: AnyRef { def self: X }](t: F[Int]): AnyRef { def self: Int } = t
def plain[F[X] <: AnyRef { def fixed: Int }](t: F[String]): AnyRef { def fixed: Int } = t
def sized[T <: AnyRef { val size: Int }](t: T): AnyRef { val size: Int } = t
def adds[T <: AnyRef { def add(n: Int): Int }](t: T): AnyRef { def add(n: Int): Int } = t
def valForDef[T <: AnyRef { val size: Int }](t: T): AnyRef { def size: Int } = t
def narrower[T <: AnyRef { def text: String }](t: T): AnyRef { def text: Any } = t
def oneOfTwo[T <: AnyRef { def self: Int; def more: Int }](t: T): AnyRef { def more: Int } = t
def read(r: AnyRef { def self: Int }): Int = r.self
def pass[T <: AnyRef { def self: Int }](t: T): Int = read(t)
def all[T <: AnyRef { def self: Int }](ts: List[T]): List[AnyRef { def self: Int }] = ts
class Box[T <: AnyRef { def self: Int }](t: T) { def got: AnyRef { def self: Int } = t }

class First[T <: AnyRef { def self: Int }] { def tag: Int = 20 }
class Second[U <: AnyRef { def self: Int }] extends First[U]
class Outer[F[X] <: AnyRef { def self: X }] { def tag: Int = 21; def use(x: Outer[F]): Int = 22 }
class Inner[G[X] <: AnyRef { def self: X }] extends Outer[G]
class Pair[F[X] <: { def o: Pair[F, G] }, G[X] <: { def o: Pair[F, G] }] { def tag: Int = 23 }
class Uno[X] { def o: Pair[Uno, Uno] = new Pair[Uno, Uno] }
class Ring[F[X] <: { def self: Ring[F] }] { def tag: Int = 24 }
class Band[F[X] <: { def self: Band[F] }] extends Ring[F]
class Link[X] { def self: Band[Link] = new Band[Link] }
def tagOf[F[X] <: { def self: Ring[F] }](r: Ring[F]): Int = r.tag
class Wide[F[X] <: Selfish] { def tag: Int = 25 }
class Narrow[G[X] <: Selfish] extends Wide[G]
class Fixed[X] { def self: Int = 1 }

class NamedLoop[X](x: X) extends Loop[X](x) with Named
class NamedFixed[X] extends Fixed[X] with Named
type Id[A] = A
def inSides[T <: AnyRef { def self: Int }, U <: T & Named](u: U): AnyRef { def self: Int } = u
def asParent[T <: AnyRef { def self: Int }, U <: T { def more: Int }](u: U): AnyRef { def self: Int } = u
def appliedInSides[G[X] <: AnyRef { def self: X }, F[X] <: G[X] & Named](t: F[Int]): AnyRef { def self: Int } = t
def throughAlias[T <: AnyRef { def self: Int }, U <: Id[T]](u: U): AnyRef { def self: Int } = u
def twoParams[T <: AnyRef { def self: Int }, V <: Named](x: T & V): AnyRef { def self: Int } = x
def refinedValue[T <: AnyRef { def self: Int }](x: T { def more: Int }): AnyRef { def self: Int } = x
class Sided[G[X] <: Selfish, H[X] <: G[X] & Named] extends Wide[H]

// A `var` of the bound: a `var` and a `def` are asked of it (a `val` is not: the errors' case).
def varOfVar[T <: AnyRef { var size: Int }](t: T): AnyRef { var size: Int } = t
def defOfVar[T <: AnyRef { var size: Int }](t: T): AnyRef { def size: Int } = t
class VarAsked[F[X] <: { var size: Int }]
class VarPassed[G[X] <: { var size: Int }] extends VarAsked[G]

@main def main(): Unit =
  val k = new Knot
  val structural = List[Any](
    keep(k).self, chain[Knot, Knot, Knot](k).self, both(new Tied).self, left[Tied](new Tied).self, aliased(k).self,
    applied[Loop](new Loop(6)).self, plain[Loop](new Loop("s")).fixed, sized(k).size, adds(k).add(3), valForDef(k).size,
    narrower(k).text, oneOfTwo(k).more, pass(k), all(List(k)).head.self, new Box(k).got.self)
  val classes = List(
    new Second[Knot].tag, new Inner[Loop].tag, new Inner[Loop].use(null), new Uno[Int].o.tag, new Band[Link].tag,
    tagOf(new Band[Link]), new Narrow[Fixed].tag)
  val inside = List[Any](
    inSides[Tied, Tied](new Tied).self, asParent[Knot, Knot](k).self, appliedInSides[NamedLoop, NamedLoop](new NamedLoop(7)).self,
    throughAlias[Knot, Knot](k).self, twoParams[Knot, Tied](new Tied).self, refinedValue[Knot](k).self,
    new Sided[NamedFixed, NamedFixed].tag)
  println(structural.mkString(" "))
  println(classes.mkString(" "))
  println(inside.mkString(" "))
