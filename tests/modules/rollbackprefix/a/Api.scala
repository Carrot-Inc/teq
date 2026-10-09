package rba

// A body withheld (`List.range`, a std member of another shape) after it wrote the prefix of
// `Lib.scala`'s top-level `value`: the next body writes the prefix again, not a reference into
// the withheld one's bytes.
object Api:
  def bad: List[Int] =
    val x = value
    List.range(0, x)
  inline def good: Int = value
