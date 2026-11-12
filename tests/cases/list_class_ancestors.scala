// `List` is a class with scala-library's class ancestors: `immutable.AbstractSeq`, `collection.AbstractSeq` and
// `AbstractIterable`. A typed pattern of one of them over a `List` is then no pattern between two unrelated
// classes (`TypeTestsCasts.checkSensical`), and a list is an instance of each.
import scala.annotation.nowarn
import scala.collection.{AbstractIterable, AbstractSeq}

@nowarn def length(xs: List[Int]): Int = xs match
  case seq: AbstractSeq[?] => seq.size
  case _ => xs.size
@nowarn def immutableLength(xs: List[Int]): Int = xs match
  case seq: scala.collection.immutable.AbstractSeq[?] => seq.size
  case _ => -1
@nowarn def iterableSize(xs: List[Int]): Int = xs match
  case it: AbstractIterable[?] => it.size
  case _ => -1

@main def run(): Unit =
  println(length(List(1, 2)))
  println(length(Nil))
  println(immutableLength(List(1, 2, 3)))
  println(iterableSize(List(1)))
  println((Nil: Any).isInstanceOf[AbstractSeq[?]])
