package tib

import tia.*

class Local extends Logging with Counted:
  println("[local]")

@main def run(): Unit =
  println(new Service().describe)
  val l = new Local
  println(l.prefix + l.count)
  println((new Logging {}).prefix)
