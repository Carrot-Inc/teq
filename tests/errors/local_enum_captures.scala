// A local enum is entered as a top-level one, its values made once, so its body cannot read
// the values around it (scalac makes the values per run of the block; see the README).
object Main:
  def f(n: Int): Int =
    enum Bad:
      case A
      def value = n
    Bad.A.value
  def main(args: Array[String]): Unit = println(f(1))
// expect: a local enum cannot refer to n: its values are made once; move the enum to the top level or make it a class
