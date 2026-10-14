package eorb

import eor.*

@main def run(): Unit =
  val c = new C(new A(new B))
  println(c.x + c.z)
  println(new E().z + inner.z)
