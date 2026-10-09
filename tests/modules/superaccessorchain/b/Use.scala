package scb

import sca.*

class Child extends Base with Stack

@main def run(): Unit =
  println(Child().f[String, String]("f"))
  println(Child().g[String, String]("g"))
  println(Child().h[String, String, String]("x", "h"))
  val p: Parent[String] = Child()
  println(p.f[String, String]("via parent"))
