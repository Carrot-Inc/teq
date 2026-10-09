// expect: modifier sealed is not allowed for this definition
// expect: modifier open is not allowed for this definition
// expect: modifier lazy is not allowed for this definition
// expect: modifier private is not allowed for this definition
// expect: modifier final is not allowed for this definition
// expect: expected 'using': the parameters of a given are using clauses
final object FO
abstract trait AT
object G:
  lazy given Int = 1
  final type FT = Int
  sealed type STy = Int
  abstract type ATy = Int
  open type OTy = Int
  lazy type LTy = Int
  lazy var lv = 1
  final var fvv = 1
  open def od = 1
  open val ov = 1
  private extension (x: Int) def p1 = x
  final extension (x: Int) def p2 = x
  sealed extension (x: Int) def p3 = x
override class OC
sealed enum SE { case A }
open enum OE { case B }
abstract enum AE { case C }
case object CO
sealed case class SCC()
lazy class LC
@main def run(): Unit = ()
class Foo2
class Bar(foo: Foo2)
object T:
  given bar(foo: Foo2): Bar = Bar(foo)
