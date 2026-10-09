// Adapted from scala3 tests/run/i7375.scala (Apache-2.0, see tests/scala3/README.md).
class Foo(private val name: String)

extension (f: Foo) def name() = "bar"

@main def Test =
  assert(Foo("foo").name() == "bar")
