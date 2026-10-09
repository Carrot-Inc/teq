class Foo
class Bar(foo: Foo)
object T:
  given bar(foo: Foo): Bar = Bar(foo)
@main def run(): Unit = println(1)
