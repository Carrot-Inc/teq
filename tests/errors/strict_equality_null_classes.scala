import scala.language.strictEquality
type Const[A] = [X] =>> A
def isNullAlias(x: Const[Any][Int]): Boolean = x == null
def isNullAny(x: Any): Boolean = x == null
def isNullRef(x: AnyRef): Boolean = x == null
def isNullBounded[T <: AnyRef](x: T): Boolean = x == null
def isNullLower[T >: Null](x: T): Boolean = x == null
def isNullString(x: String): Boolean = x != null
// Under strict equality `null` compares with a type one of whose class symbols is Null or derives from Object
// (Synthesizer.canComparePredefinedClasses), else through a given CanEqual: not Any, a curried alias of it or a
// type parameter bounded below by Null, as scalac 3.8.4 reports.
// expect: 3:48: error: values of types Const[Any][Int] and Null cannot be compared with == or !=
// expect: 4:34: error: values of types Any and Null cannot be compared with == or !=
// expect: 7:45: error: values of types T and Null cannot be compared with == or !=
// expect: 3 errors found
