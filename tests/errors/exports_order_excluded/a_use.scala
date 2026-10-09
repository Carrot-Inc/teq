// expect: not found: cls
// expect: value some is not a member of Int
// The consumer is entered and completed first (a header looks through the import while the
// facade is not complete); the names the facade's wildcards exclude stay out in every order.
package app

import util.Prelude.*

trait Base

class Model extends Base:
  def bad: Option[Int] = 1.some

@main def run(): Unit =
  println(cls)
