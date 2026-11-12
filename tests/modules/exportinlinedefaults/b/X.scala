package eidu

import eid.*

// Each call leaves out a parameter whose default
// is an inline getter: the forwarder's getter is
// to be expanded, and in it the member's.
@main def run(): Unit =
  println(Fw.p())
  println(Fw.q(2)())
  println(Fw.t())
