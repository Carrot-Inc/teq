//> using platform jvm
// split-watch: jvm
// jars: lazy-inline-lib
// Expansions of a scalac-compiled library's inline methods that declare a local lazy val (tests/support/
// lazy_inline_lib.scala): dotty freshens each expansion's definitions in the unit holding it, so each keeps its own
// holder, accessor and initialiser (`x$lzyN$1`, `x$lzyINITM$1` numbered in this file), none shared or dropped; an
// expanded local class with lazy vals in two methods and a lazy member, each copy numbered in this file.
import lazyinline.Library

class A:
  def go(): Unit =
    println(Library.value(1))
    println(Library.value(2))
    val a = Library.reader(3)
    val b = Library.reader(4)
    println(a())
    println(b())
    println(Library.local(1))
    println(Library.local(3))

@main def run(): Unit = new A().go()
