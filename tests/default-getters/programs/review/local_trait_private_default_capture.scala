// A local trait's private member whose default reads a value of the block (the native session's grid, 25 Nov): the
// call of the member in the trait takes the default from its getter, which reads the captured value, on every target,
// as scalac 3.8.4 prints it. teq's JVM output declared the private member's getter under the member's expanded name
// in the trait and implemented it under the plain one in the anonymous class mixing the trait in, so the trait's
// call met no implementation (`AbstractMethodError`).
object Main {
  def main(args: Array[String]): Unit = {
    val x: Long = args.length.toLong + 7L
    val y: Long = args.length.toLong + 1L
    trait K {
      private def f(p0: Long, last: Long = y): Long = x + p0 + last
      def g: Long = f(1L)
    }
    println((new K {}).g)
  }
}
