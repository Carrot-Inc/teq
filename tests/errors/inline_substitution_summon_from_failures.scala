// expect: 21:11: error: cannot reduce summonFrom with
// expect:  patterns :  case a: A
// expect: 23:11: error: ambiguous given instances for A: a1, a2
// expect: 2 errors found
// A search of `summonFrom` that is ambiguous reports and stops the reduction, as scalac's
// `InlineReducer` does ("Ambiguous given instances: both given instance a1 in object X and given
// instance a2 in object X match type A"), where one that finds nothing goes on to the next case
// (`pick` without the import prints `none`); no case left is `cannot reduce summonFrom`. The
// retype path takes the ambiguous search for one that finds nothing and prints `none`.
import scala.compiletime.summonFrom
trait A
object X:
  given a1: A = new A {}
  given a2: A = new A {}
inline def pick: String = summonFrom {
  case a: A => "A"
  case _ => "none"
}
inline def only: String = summonFrom { case a: A => "A" }
@main def run(): Unit =
  println(only)
  import X.given
  println(pick)
