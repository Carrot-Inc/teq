package grrb

import grra.*

@main def run(): Unit =
  import O.given
  given b: B = new B
  println(summon[A].isInstanceOf[B])
