// scala.util.hashing.MurmurHash3, whose functions a library's Hash instances call.
import scala.util.hashing.MurmurHash3
final case class Pair(n: Int, s: String)
object Main:
  def main(args: Array[String]): Unit =
    val s = MurmurHash3.seqSeed
    println(MurmurHash3.mix(s, 7) + " " + MurmurHash3.mixLast(1, 2) + " " + MurmurHash3.finalizeHash(5, 3))
    println(MurmurHash3.orderedHash(List(1, 2, 3)) + " " + MurmurHash3.orderedHash(List(1, 2, 4), s) + " " + MurmurHash3.orderedHash(List(1, 2), s) + " " + MurmurHash3.orderedHash(Nil, s) + " " + MurmurHash3.orderedHash(List(9), s))
    println((MurmurHash3.orderedHash(List(1, 2, 3), s) == List(1, 2, 3).hashCode).toString + " " + (MurmurHash3.orderedHash(Vector("a", "c", "b"), s) == Vector("a", "c", "b").hashCode))
    println((MurmurHash3.unorderedHash(List(3, 1, 2), MurmurHash3.setSeed) == Set(1, 2, 3).hashCode).toString + " " + MurmurHash3.unorderedHash(List(3, 1, 2)))
    println((MurmurHash3.rangeHash(1, 1, 3, s) == List(1, 2, 3).hashCode).toString + " " + MurmurHash3.rangeHash(2, 3, 8))
    println((MurmurHash3.caseClassHash(Pair(1, "a")) == Pair(1, "a").hashCode).toString + " " + (MurmurHash3.productHash(Pair(1, "a"), MurmurHash3.productSeed) == Pair(1, "a").hashCode) + " " + (MurmurHash3.listHash(List(4, 5), s) == List(4, 5).hashCode) + " " + (MurmurHash3.seqHash(List(1, 3, 2)) == List(1, 3, 2).hashCode))
    println(MurmurHash3.stringHash("hello") + " " + MurmurHash3.productSeed + " " + s + " " + MurmurHash3.mapHash(Map.empty[Int, Int]))
    println((MurmurHash3.setHash(Set(1, 2)) == Set(2, 1).hashCode).toString + " " + (MurmurHash3.mapHash(Map(1 -> "a", 2 -> "b")) == Map(2 -> "b", 1 -> "a").hashCode) + " " + (MurmurHash3.indexedSeqHash(Vector(1, 2, 3), s) == Vector(1, 2, 3).hashCode))
    println(MurmurHash3.finalizeHash(MurmurHash3.mix(MurmurHash3.mix(MurmurHash3.productSeed, "Pair".hashCode), 1), 2))
