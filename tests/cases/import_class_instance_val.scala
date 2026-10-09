// An import whose prefix is a stable val of the enclosing class's instance (dotty's
// `Typer.typedImport` and `Checking.checkStable`): the members it brings are read on that
// instance, `this.a.m`, from the class's body, a nested class's and a lambda's, through named,
// renamed, wildcard and given selectors, and through a val the class inherits (its type seen
// from the class, `X` of a generic parent's) or a parameter. A named selector may name a type
// member, a nested class or an extension of the value's class.
class API(val value: Int):
  def twice: Int = value * 2
  given Int = value + 100
  type Unit2 = (Int, Int)
  class Box(val n: Int)
  extension (n: Int) def tripled: Int = n * 3

class Base:
  val inherited = new API(1000)

trait Enriched[X]:
  val value: X

trait PairOps[A] extends Enriched[(A, A)]:
  def swapped: (A, A) =
    import value.*
    (_2, _1)

class Use(val a: API, p: API) extends Base:
  import a.*
  import a.given
  import a.{value as renamed}
  import p.{value as fromParam}
  import inherited.{twice as inheritedTwice}
  import a.{Unit2, Box, tripled}
  def members: Unit2 = (value, value.tripled)
  def noBox: Option[Box] = None
  def read: Int = value
  def doubled: Int = twice
  def summoned: Int = summon[Int]
  def same: Boolean = renamed == value
  def param: Int = fromParam
  def parent: Int = inheritedTwice
  class Inner:
    def outerValue: Int = value + 1
  def viaLambda: List[Int] = List(1, 2).map(_ + value)

@main def run(): Unit =
  val u1 = new Use(new API(7), new API(70))
  val u2 = new Use(new API(9), new API(90))
  println(List(u1.read, u2.read, u1.doubled, u2.doubled, u1.summoned, u2.summoned))
  println(List(u1.same, u2.same))
  println(List(u1.param, u2.param, u1.parent))
  println(List(u1.members, u2.members, u1.noBox.isEmpty))
  println(List(new u1.Inner().outerValue, new u2.Inner().outerValue))
  println(u1.viaLambda ++ u2.viaLambda)
  val pair = new PairOps[Int] { val value = (1, 2) }
  println(pair.swapped)
