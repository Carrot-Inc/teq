// The value of a path through an export's forwarder of an inline val (`ValueOf` of `width.type`): the
// forwarder is an inline method, and the value is the member's constant, read unqualified inside the exporting
// object, qualified from outside, through a second export, through a renamed one, and as a using argument.
package paths

object Origin:
  inline val width = 3

object Facade:
  export Origin.width
  def within: Int = summon[ValueOf[width.type]].value

object Second:
  export Facade.width

object Renamed:
  export Origin.{width as breadth}
  def within: Int = valueOf[breadth.type]

def same[T <: Singleton](x: T)(using v: ValueOf[T]): Boolean = x == v.value
def held(using v: ValueOf[Facade.width.type]): Int = v.value

@main def run(): Unit =
  println(s"${Facade.within} ${summon[ValueOf[Facade.width.type]].value} ${Facade.width}")
  println(s"${valueOf[Second.width.type]} ${Renamed.within} ${valueOf[Renamed.breadth.type]} ${Renamed.breadth}")
  println(s"${same(Facade.width)} $held ${summon[ValueOf[Facade.width.type]].value == Facade.width}")
