package captureorder

import captureorder.marks.*

// What a class of a block captures after the inlining phase is appended class by class, the classes it declares
// taken by their place: the module built alone gives the class the constructor the whole build gives it.
object Main:
  // Each read alone in a class of its own, the outer class's first.
  def flat(n: Int): Int =
    given a: First = new First(n)
    given b: Second = new Second(n + 1)
    given c: Third = new Third(n + 2)
    given d: Fourth = new Fourth(n + 3)
    class Outer:
      def own: Int = Marks.first
      class Z { def get: Int = Marks.second }
      class Y { def get: Int = Marks.third }
      class X { def get: Int = Marks.fourth }
      def get: Int = own + new Z().get + new Y().get + new X().get
    new Outer().get
  // A class two deep read before its owner's own read, the outer class's read last.
  def deep(n: Int): Int =
    given a: First = new First(n)
    given b: Second = new Second(n + 1)
    given c: Third = new Third(n + 2)
    given d: Fourth = new Fourth(n + 3)
    class Outer:
      class Z:
        class Deep { def get: Int = Marks.fourth }
        def get: Int = Marks.third + new Deep().get
      class Y { def get: Int = Marks.second }
      def get: Int = Marks.first + new Z().get + new Y().get
    new Outer().get
  // The owner's own read before the class two deep.
  def deepAfter(n: Int): Int =
    given a: First = new First(n)
    given b: Second = new Second(n + 1)
    given c: Third = new Third(n + 2)
    given d: Fourth = new Fourth(n + 3)
    class Outer:
      class Z:
        def get: Int = Marks.third + new Deep().get
        class Deep { def get: Int = Marks.fourth }
      class Y { def get: Int = Marks.second }
      def get: Int = Marks.first + new Z().get + new Y().get
    new Outer().get
  def main(args: Array[String]): Unit =
    println(flat(10))
    println(deep(20))
    println(deepAfter(30))
