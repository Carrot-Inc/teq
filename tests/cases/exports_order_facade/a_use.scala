// The files are named so that this consumer is entered and completed first: the headers below
// look through the import while the facade, its providers and the package object are not
// complete yet. The facade exports what an object inherits from traits of a later file (an
// extension, a given whose receiver is the object), a renamed member next to an excluded one,
// and what a package object inherits from its parent.
package app

import util.Prelude.{*, given}

trait Base

class Model extends Base:
  def shown: String = summon[lib.Show[Int]].show(1)

class Handler(e: ReactEvent):
  def name: String = e.kind

@main def run(): Unit =
  println(Model().shown)
  println(4.twice)
  println(mainTag)
  println(Handler(event("click")).name)
  println(named("x"))
  println(lib.Impl.count)
