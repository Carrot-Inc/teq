// An opaque alias of Long read before its initialiser runs: the read is its underlying type's zero.
object Types { opaque type L = Long; def make(n: Long): L = n; def add(n: L): Long = n + 1L }
class A { def read: Types.L = later; val early: Long = Types.add(read); val later: Types.L = Types.make(21L) }
object Main { def main(args: Array[String]): Unit = println((new A).early) }
