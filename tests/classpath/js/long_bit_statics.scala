// jars: scala-library
// std: scala-library
// targets: js jvm
// java.lang.Long's statics that scala-library's collections reach, over values read at run time: `highestOneBit`
// (LongMap's branching bit), `lowestOneBit`, `reverseBytes` (`hashing.byteswap64`), `toOctalString` (RichLong's)
// and `hashCode`, the JDK's `(int)(l ^ (l >>> 32))` rather than `##`.
@main def run(args: String*): Unit =
  val k = args.length
  val longs = List(0L, 1L, -1L, 37L, 0x123456789abcdefL, Long.MinValue, Long.MaxValue).map(_ + k)
  for l <- longs do
    import java.lang.Long.{hashCode as hash, highestOneBit, lowestOneBit, reverseBytes, toOctalString}
    println(s"$l ${highestOneBit(l)} ${lowestOneBit(l)} ${reverseBytes(l)} ${toOctalString(l)} ${hash(l)}")
  println(scala.collection.immutable.LongMap(1L + k -> "a", (1L << 40) -> "b", -5L -> "c"))
  println((scala.collection.immutable.LongMap(1L + k -> 2) ++ scala.collection.immutable.LongMap(9L -> 3)).updated(Long.MinValue, 1).toList)
  println(scala.util.hashing.byteswap64(0x0102030405060708L + k))
  println((12345L + k).toOctalString)
