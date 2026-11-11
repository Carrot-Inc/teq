// Covariant default methods inherited from jars need bridges even when their
// broader descriptor already has a concrete body. An abstract superclass owns
// the bridges; its concrete descendants inherit them.
package jarcovariantfactory

abstract class SequenceBase extends IndexedSeq[Int]:
  def apply(i: Int): Int = i * 3

class Sequence extends SequenceBase:
  def length: Int = 3

final class SequenceChild extends Sequence

object Main:
  def main(args: Array[String]): Unit =
    for xs <- List(new Sequence, new SequenceChild) do
      println(xs.map(_ + 1))
      println(xs.reverse)
