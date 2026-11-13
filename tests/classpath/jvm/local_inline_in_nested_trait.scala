// A block's inline method is called in a trait nested in a class of the block. The JVM's classes take
// nothing for a block's inline method: the program prints what scalac's does.
package blocks

@main def run(): Unit =
  inline def twice(x: Int): Int = x * 2
  class Outer:
    trait Inner:
      def half = twice(4)
    object Made extends Inner
  println(Outer().Made.half)
