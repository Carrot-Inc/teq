package gbc

import gbl.*

class C extends P { override def m(a: V = new V(7)): String = "C" + a }
class E extends C { def go: String = super.m() }

@main def run(): Unit =
  println(new P().m())
  println(new C().m(new V(5)))
  println(new C().m())
  println((new C(): P).m())
  println(new E().go)
