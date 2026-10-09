// A guard of an inline match's case or of a `summonFrom` case reduces on its constant value as
// scalac 3.8.4's `InlineReducer` reads it (`ConstantValue`): the tree's, or its type's, a call of
// `def yes(): true` true and one of `def no(): false` false, which gives way to the next case.
import scala.compiletime.summonFrom
def yes(): true = true
def no(): false = false
inline def pick(x: Int): Int = inline x match
  case _ if no() => 0
  case _ if yes() => 1
  case _ => 2
inline def choose: Int = summonFrom {
  case _ if no() => 0
  case _ if yes() => 1
  case _ => 2
}
@main def main(): Unit =
  println(pick(3))
  println(choose)
