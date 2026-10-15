package rba

// A body withheld (`IndexedSeq.range`, a std member of another shape) after it wrote the prefix of
// `Lib.scala`'s top-level `value`: the next body writes the prefix again, not a reference into
// the withheld one's bytes.
object Api:
  def bad: List[Int] =
    val x = value
    IndexedSeq.range(0, x).toList
  inline def good: Int = value
