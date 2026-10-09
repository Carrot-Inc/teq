package spa

// An expansion in a parent's call whose binding `x` stands beside the constructor parameter `x`
// that the expansion still reads.
object F:
  transparent inline def add(x: Int, inline y: Int): Int = x + y

class Parent(val n: Int)
class Child(n: Int, x: Int) extends Parent(F.add(n + 1, x))
