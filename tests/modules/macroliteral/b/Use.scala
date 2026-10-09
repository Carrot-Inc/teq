package mlb

import mla.Email
import mla.Literals.*

@main def run(): Unit =
  println(email" Ops+Desk@Example.ORG ".toStr)
  println(Email.shout(email"a@b.c"))
  println(Email.fromString("nobody"))
  println(Email.fromStringEither("X@Y").map(_.toStr))
  println(mla.Audit.count)
