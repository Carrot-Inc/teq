// Enums: an enumeration-shaped one, one with a class case, a generic one.
package probe.enums

enum Simple:
  case A, B
enum Mixed(val code: Int):
  case X extends Mixed(1)
  case Y(n: Int) extends Mixed(n)
enum Tree[+T]:
  case Leaf(v: T)
  case Node(l: Tree[T], r: Tree[T])
  case Empty
