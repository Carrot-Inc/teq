package gbb

import gba.*

class C extends Base with Gen[Int]

class Local:
  def d: String = "d"
  val v: String = "v"
  given g: String = "g"
  given gp(using s: String): String = s + "!"
  var x: String = "x"
class C2 extends Local with Gen[String]

@main def run(): Unit =
  val c: Gen[Int] = C()
  c.x = c.x + 1
  println(c.d + c.v + c.g + c.gp(using "abcd") + c.x)
  val c2: Gen[String] = C2()
  c2.x = c2.x + "y"
  println(c2.d + c2.v + c2.g + c2.gp(using "s") + c2.x)
