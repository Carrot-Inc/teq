// `referencesName`'s fold, child by child (the review of `50e3a1b38`): an import's qualifier alone, not its selector's
// `given` bound (`ImportBound`: the evidence after the parameter); a postfix operator an identifier (`Postfix`: the
// evidence named `size` before it); `{ case .. }` no selector (`CaseLambda`: an evidence named `x$1` after it).
import scala.language.postfixOps
class ann(val x: Any) extends scala.annotation.StaticAnnotation
object ord
object Hold { given ord.type = ord }
type ImportBound = [X: Ordering as ord] => (Int @ann({ import Hold.given ord.type; 0 })) => Int
type Postfix = [X: Ordering as size] => (Int @ann(List(1) size)) => Int
type CaseLambda = [X: Ordering as x$1] => (Int @ann({ case 1 => 2 }: PartialFunction[Int, Int])) => Int
object Test {
  def main(args: Array[String]): Unit = {
    val importBound: ImportBound = [X] => (x: Int) => (o: Ordering[X]) ?=> x + 1
    val postfix: Postfix = [X] => (o: Ordering[X]) ?=> (x: Int) => x + 2
    val caseLambda: CaseLambda = [X] => (x: Int) => (o: Ordering[X]) ?=> x + 3
    println(importBound[Int](1)(using Ordering.Int))
    println(postfix[Int](using Ordering.Int)(1))
    println(caseLambda[Int](1)(using Ordering.Int))
  }
}
