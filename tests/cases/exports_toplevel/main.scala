package app

import lib.*
import lib.given

@main def run(): Unit =
  println(exclaim("hi"))
  println(List("a", "b").mkString(separator))
  println("ab".twice)
  val v: Version = Version(1, 2)
  println(show(v))
  println(summon[Pretty[Version]].pretty(Version(3, 4)))
  println(twiceOf(21))
  println(lib.exclaim("qualified"))
  println(lib.Version(5, 6))
  val w: lib.Version = lib.Version(7, 8)
  w match
    case Version(major, _) => println(major)
  println(facade.Facade.exclaim("facade"))
  println(facade.Facade.Version(9, 9))
  println(facade.Facade.twiceOf(4))
