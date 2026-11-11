package eidu

import eid.*

// Each call leaves out a parameter whose default is an inline getter: the forwarder's getter is expanded, and in
// it the member's. Built over teq's products these threw NoSuchMethodError on Box.p$default$1, Box.q$default$2
// and Impl$.t$default$1, the member's getter called in place of expanded.
@main def run(): Unit =
  println(Fw.p())
  println(Fw.q(2)())
  println(Fw.t())
