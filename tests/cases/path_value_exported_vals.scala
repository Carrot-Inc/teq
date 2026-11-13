// `ValueOf` and the singleton type test of paths through exports of members that are no inline vals: a
// val, a final val, an object, an enum's case, and a val that is the prefix of a class and of a deeper path.
package paths

enum Shade:
  case Dark, Light

class Outer(val k: Int):
  class In(val n: Int):
    def sum: Int = n + k

class Row:
  val cells = 3

object Origin:
  val plain: String = "p"
  val other: String = "q"
  final val fixed = 4
  val outer = Outer(10)
  val row = Row()
  object Inner:
    val n = 7

object Facade:
  export Origin.{plain, other, fixed, outer, row, Inner}
  export Shade.Dark
  def within: String = summon[ValueOf[plain.type]].value

def kind(x: Any): String = x match
  case _: Facade.plain.type => "plain"
  case _: Facade.Inner.type => "inner"
  case _ => "none"

def second(x: Any): String = x match
  case _: Facade.other.type => "other"
  case _ => "none"

@main def run(): Unit =
  println(s"${Facade.within} ${valueOf[Facade.plain.type]} ${valueOf[Facade.fixed.type]}")
  println(s"${kind(Origin.plain)} ${second(Origin.other)} ${kind(Origin.Inner)} ${kind(Origin.other)}")
  println(s"${valueOf[Facade.Inner.type].n} ${valueOf[Facade.Dark.type] == Shade.Dark} ${Facade.Dark == Shade.Dark}")
  val made = new Facade.outer.In(5)
  println(s"${made.sum} ${valueOf[Facade.outer.type].k} ${valueOf[Facade.row.cells.type]} ${valueOf[Facade.row.type].cells}")
