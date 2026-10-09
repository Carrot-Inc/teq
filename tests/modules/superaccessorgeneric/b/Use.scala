package sgb

import sga.*

class Child extends Base with Stack

class GenChild extends Gen[String] with Stack

class TwiceInts extends Ints with Twice[Int]

@main def run(): Unit =
  println(Child().id("ok"))
  println(Child().pair("a", 1))
  println(Child().narrow("narrow"))
  println(GenChild().narrow("gen narrow"))
  println(GenChild().id("gen"))
  println(TwiceInts().id(40))
  val p: Parent[String] = Child()
  println(p.id("via parent"))
