// Adapted from scala3 tests/run/i6716.scala (Apache-2.0, see tests/scala3/README.md).
//> using options -Werror

trait Monad[T]:
  def id: String
class Foo
object Foo {
  given Monad[Foo] { def id = "Foo" }
}

opaque type Bar = Foo
object Bar {
  given Monad[Bar] = summon[Monad[Foo]] // was error, fixed by given loop prevention
}

@main def Test =
  assert(summon[Monad[Foo]].id == "Foo")
  assert(summon[Monad[Bar]].id == "Foo")
