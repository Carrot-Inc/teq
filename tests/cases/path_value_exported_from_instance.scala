// `ValueOf` of paths through exports from an instance: in an object, in a class through `this` and through a
// val that holds the instance, with an inline val and a plain val behind the forwarders; and an object of
// the instance, exported, as the prefix of a val and of a class.
package paths

class Holder:
  inline val width = 3
  val plain = 4
  object Nest:
    val deep = 7
    class Deep:
      def n: Int = deep

object Facade:
  val holder = Holder()
  export holder.{width, Nest}
  def within: Int = valueOf[width.type]

class Wide:
  val holder = Holder()
  export holder.{width, plain}
  def within: Int = valueOf[width.type]

@main def run(): Unit =
  val w = Wide()
  println(s"${Facade.within} ${valueOf[Facade.width.type]} ${w.within}")
  println(s"${valueOf[w.width.type]} ${valueOf[w.plain.type]} ${w.width} ${w.plain}")
  println(s"${valueOf[Facade.Nest.deep.type]} ${(new Facade.Nest.Deep).n} ${valueOf[Facade.Nest.type].deep} ${Facade.Nest.deep}")
