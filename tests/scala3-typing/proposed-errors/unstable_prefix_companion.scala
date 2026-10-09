// expect: type mismatch: found String, required Bar
// expect: type mismatch: found String, required Bar
// expect: type mismatch: found String, required Bar
// scala3's neg/i15939: the companion of a class nested in a class enters the implicit scope of
// the class's type only through a stable prefix. `mkFoo.andThen(mkBarString)` wants a
// `mkFoo.Bar` whose prefix `mkFoo` is a method call, so its companion's conversion
// `Bar.fromString` is out of scope and scalac rejects the call; teq reaches the companion
// through the class (`dependent_prefix`, the implicit scope walk of `implicits.rs`), as it does
// for a stable prefix, and accepts it. The check is the prefix's stability where a nested
// class's companion enters the scope, medium in reach: the same walk serves skunk's and
// twiddles' companions behind stable paths.
import scala.language.implicitConversions

object Test:
  class Foo:
    class Bar:
      override def toString() = "bar"
    object Bar:
      implicit def fromString(a: String): Bar = new Bar
    def andThen(b: Bar): Unit = println(s"use $b")
    def andThen_:(b: Bar): Unit = println(s"use $b")
    def andThenByName_:(b: => Bar): Unit = println(s"use $b")

  def mkFoo: Foo = ???
  def mkBarString: String = ???
  mkFoo.andThen(mkBarString)
  mkBarString andThen_: mkFoo
  mkBarString andThenByName_: mkFoo
