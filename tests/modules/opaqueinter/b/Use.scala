package oib

import oia.*

def describe(n: Named): String = n.greet
def both(x: Named & Sized): String = s"${x.name}:${x.size}"
def firstOf[T](s: Source[T]): T = s.items.head

@main def run(): Unit =
  val h = Store.handle("cup", 3)
  println(h.name)
  println(h.size)
  println(describe(h))
  println(both(h))
  val p = Store.pack(1, 2, 3)
  println(p.items.map(_ + 1))
  println(p.count)
  println(firstOf(p))
