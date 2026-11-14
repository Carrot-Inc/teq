package app

import renamed.*

class Mine(val x: Int) extends AnyVal with Loose

@main def run(): Unit =
  def show(label: String, a: Any, b: Any, c: Any): Unit =
    println(s"$label ${a == b} ${a == c} ${a.equals(b)} ${a.hashCode == b.hashCode}")
  show("Below", new Below(1), new Below(1), new Below(2))
  println(Set(new Below(1), new Below(1)).size)
  show("Deep", new Deep(1), new Deep(1), new Deep(2))
  show("Hashed", new Hashed(1), new Hashed(1), new Hashed(2))
  println(new Hashed(1).hashCode(3))
  show("Mine", new Mine(1), new Mine(1), new Mine(2))
  println(Set(new Mine(1), new Mine(1)).size)
