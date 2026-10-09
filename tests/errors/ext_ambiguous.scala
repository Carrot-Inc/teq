// expect: ambiguous extension methods: both Foo and given_Ops_Foo provide show on Foo
class Foo
trait Ops[A]:
  extension (a: A) def show: String
object Foo:
  extension (f: Foo) def show = "companion extension"
  given Ops[Foo] with
    extension (a: Foo) def show = "given in companion"

@main def run(): Unit =
  println(Foo().show)
