// `ValueOf` of paths through an export of inline vals of each constant kind.
package paths

object Origin:
  inline val name = "w"
  inline val on = true
  inline val ratio = 1.5
  inline val mark = 'c'
  inline val big = 5L
  inline val small = 2.5f

object Facade:
  export Origin.*

@main def run(): Unit =
  println(s"${valueOf[Facade.name.type]} ${valueOf[Facade.on.type]} ${valueOf[Facade.ratio.type]} ${valueOf[Facade.mark.type]} ${valueOf[Facade.big.type]}")
  val small: Float = valueOf[Facade.small.type]
  println(s"${small + 1} ${valueOf[Facade.small.type]}")
