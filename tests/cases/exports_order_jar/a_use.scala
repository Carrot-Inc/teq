//> using dep org.typelevel::cats-core:2.13.0
// jars: scala-library cats-kernel cats-core
// The files are named so that this consumer is entered and completed first: Model's header
// looks through `import util.Prelude.*` while Prelude and the jar traits behind it are not
// complete yet. The syntax of cats reaches it through an object of a jar trait, and members of
// a jar's object come renamed.
package app

import util.Prelude.{*, given}

trait Base

class Model extends Base:
  def pair(x: Int): Option[Int] = x.some

@main def run(): Unit =
  println(Model().pair(1))
  println("left".asLeft[Int])
  println(List(1, 2, 3).traverse(x => Option(x)))
  println(nel(1, 2, 3))
  println(single("one").length)
