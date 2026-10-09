// jars: scala-library
// scala.util.hashing.MurmurHash3 is the std's, which binds before the jar's object of the
// same name, and gives the values the jar's bodies would.
import scala.util.hashing.MurmurHash3
object Main:
  def main(args: Array[String]): Unit =
    println(MurmurHash3.stringHash("hello"))
    println(MurmurHash3.stringHash(""))
    println(MurmurHash3.mix(1, 2))
    println(MurmurHash3.mixLast(1, 2))
    println(MurmurHash3.finalizeHash(3, 2))
    println(MurmurHash3.orderedHash(List(1, 2, 3)))
    println(MurmurHash3.orderedHash(List(1, 3, 9, 27)))
    println(MurmurHash3.unorderedHash(List("a", "b", "c")))
    println(MurmurHash3.arrayHash(Array(1, 2, 3)))
    println(MurmurHash3.arrayHash(Array("x", "y")))
    println(MurmurHash3.rangeHash(1, 2, 9))
    println(MurmurHash3.bytesHash(Array[Byte](1, 2, 3, 4, 5)))
    println(MurmurHash3.stringHash("a longer string with an odd length!"))
