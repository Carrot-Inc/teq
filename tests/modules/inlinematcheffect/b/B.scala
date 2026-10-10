package imeb

import imea.*

var n = 0
def tick(): Int = { n += 1; 1 }

@main def run(): Unit =
  println(typeOnly(tick()))
  println(byName(tick()))
  println(n)
