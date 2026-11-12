// Expansions of inline methods declaring local lazy vals, from another file of the build: each expansion its own
// holder, accessor and initialiser, numbered in this file beside its own lazy val, in a class's method and at the top
// level; an expanded local class with lazy vals in two methods and a lazy member, each copy numbered in this file.
import lib.Library

class A:
  def go(): Unit =
    lazy val x = 100
    println(Library.value(1))
    println(Library.value(2))
    val a = Library.reader(3)
    val b = Library.reader(4)
    println(a())
    println(b())
    println(Library.twice(5) + x)
    println(Library.local(1))
    println(Library.local(3))

def top(): Int = Library.value(10) + Library.twice(20)

@main def run(): Unit =
  new A().go()
  println(top())
