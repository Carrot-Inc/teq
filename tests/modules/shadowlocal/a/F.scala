package sla

// A local class inside a parent's call whose own parent's call expands with a binding `x`
// beside its constructor parameter `x`, which the expansion still reads.
object F:
  transparent inline def add(x: Int, inline y: Int): Int = x + y

class Parent(val n: Int)
class Child(n: Int, x: Int) extends Parent({ class Inner(n: Int, x: Int) extends Parent(F.add(n + 1, x)); new Inner(n, x).n })
