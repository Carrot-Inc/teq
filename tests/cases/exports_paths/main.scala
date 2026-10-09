package app

import lib.*

trait Base:
  export lib.internal.Colors

// paths that start with an inherited export and with a name from a wildcard export
object Api extends Base:
  export Colors.red
  export lib.internal.{Colors as _, *}
  export Icons.home

object Numbers:
  def one: Int = 1
  def two: Int = 2

// a rename hides the name from a wildcard of the same clause only
object Renames:
  export Numbers.{one as uno}
  export Numbers.*

object SameClause:
  export Numbers.{one as uno, *}

@main def run(): Unit =
  println(Icon("x"))
  println(homeIcon)
  println(Icons.home)
  println(Api.red)
  println(Api.home)
  println(Api.Colors.red)
  println(Renames.uno + Renames.one + Renames.two)
  println(SameClause.uno + SameClause.two)
