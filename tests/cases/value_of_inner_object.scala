// `ValueOf` of an object nested in a class, the type its argument gives a type parameter (`plain(Tar)` in the
// class, `plain(spec.Tar)` outside it): the module's reference through its prefix, as dotty's
// `Synthesizer.synthesizedValueOf` makes it of a module class's `TypeRef`, the enclosing instance's object inside
// the class and the path's outside; an inline method's `using inline` evidence alike (Airstream's `handleValue`).
final class Box[V](val v: V)
object Ops:
  def plain[V](v: V)(using valueOf: ValueOf[V]): V = valueOf.value
  inline def handleValue[V](inline v: V)(using inline valueOf: ValueOf[V]): Box[V] = new Box(valueOf.value)

class Spec(val label: String):
  sealed trait Foo
  case object Tar extends Foo
  object Plain
  def run(): Unit =
    println(Ops.plain(Tar))
    println(Ops.handleValue(Tar).v)
    println(Ops.plain(Plain) eq Plain)
    println(summon[ValueOf[Tar.type]].value == Tar)

@main def main(): Unit =
  val spec = new Spec("one")
  spec.run()
  println(Ops.plain(spec.Tar))
  println(Ops.plain(spec.Plain) eq spec.Plain)
