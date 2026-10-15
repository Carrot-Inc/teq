// An `unapplySeq` whose result is a sequence itself, no `Option` around it, as tyrian-next's
// `Batch.unapplySeq` returns a `Seq`: the pattern matches the elements, a fixed number or a rest,
// a pattern of another length falls through to the next case, and the extractor cannot fail, so
// a rest alone covers the scrutinee (`SpaceEngine.isIrrefutable`).
sealed trait Batch[+A]:
  def toList: List[A]
object Batch:
  final case class Wrapped[A](values: List[A]) extends Batch[A]:
    def toList: List[A] = values
  def apply[A](values: A*): Batch[A] = Wrapped(values.toList)
  def unapplySeq[A](b: Batch[A]): Seq[A] = b.toList

object Pair:
  def unapplySeq(s: String): List[String] = s.split(",").toList

def describe(b: Batch[Int]): String = b match
  case Batch() => "empty"
  case Batch(a) => "one " + a
  case Batch(a, b, c) => "three " + (a + b + c)
  case _ => "other"

def total(b: Batch[Int]): Int = b match
  case Batch(all*) => all.sum

@main def run(): Unit =
  println(describe(Batch()))
  println(describe(Batch(4)))
  println(describe(Batch(1, 2, 3)))
  println(describe(Batch(1, 2)))
  println(total(Batch(1, 2, 3, 4)))
  "x,y" match
    case Pair(a, b) => println(b + a)
    case _ => println("no pair")
