// A type parameter bounded by an intersection conforms to a class through either side
// (scala-library's `LinearSeqOps[+A, +CC[X] <: LinearSeq[X], +C <: LinearSeq[A] & LinearSeqOps[A, CC, C]]`,
// whose `lengthCompare` izumi's `Tag` macro runs): `C` is an `Ops[A, CC, C]` by its second side.
trait LSeq[+A] extends Ops[A, LSeq, LSeq[A]]
trait Ops[+A, +CC[X] <: LSeq[X], +C <: LSeq[A] & Ops[A, CC, C]]:
  def tail: C
  def isEmpty: Boolean
  def count(): Int =
    var s = this
    var n = 0
    while !s.isEmpty do
      s = s.tail
      n += 1
    n

final class Cells(n: Int) extends LSeq[Int]:
  def tail: LSeq[Int] = Cells(n - 1)
  def isEmpty: Boolean = n <= 0

@main def main(): Unit = println(Cells(3).count())
