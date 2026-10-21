// A polymorphic function's evidences, placed as dotty's `elimContextBounds` places them: before the parameters where
// a parameter's type names one anywhere in its tree (`referencesName`: a singleton, a refinement, a match type, an
// annotation's argument, an operator), joined into a contextual clause with the function's own parameters named
// `x$n` (`makePolyFunctionType`), a literal's empty clause dropped. A binder's annotation is typed where the binder
// stands (`annotContext`): a method's parameter, a block's local, the clause's own binders where a type definition
// owns them. The review of `16412b2e9`.
import scala.annotation.StaticAnnotation

class ann(val n: Any) extends StaticAnnotation
class tagged[T] extends StaticAnnotation
extension (i: Int) def ord(j: Int): Int = i + j

type Singleton = [X: Ordering as ord] => ord.type => Int
type Contextual = [X: Ordering as ord] => X ?=> ord.type
type Annotated = [X: Ordering as ord] => (Int @ann(ord)) => Int
type Operator = [X: Ordering as ord] => (Int @ann(1 `ord` 2)) => Int
type Refined = [X: Ordering as ord] => ({ val y: ord.type }) => Int
type Matched = [X: Ordering as ord] => (ord.type match { case Any => Int }) => Int
type EmptyType = [X: Ordering] => () => Int
type Synthetic = [X] => Int => x$1.type

def local(x: Int): Int = {
  type L = [@ann(x) A] =>> List[A]
  val l: L[Int] = List(x)
  l.head
}
def literal(x: Int) = [@ann(x) A] => (a: A) => a
def kinded[T, F[@tagged[T] A]](f: F[T]) = f
type Siblings = [@tagged[B] A, B] =>> (A, B)
trait SiblingKinds[F[@tagged[B] A, B]]

object Test {
  def main(args: Array[String]): Unit = {
    val singleton: Singleton = [X] => (ord: Ordering[X]) ?=> (x: ord.type) => 1
    val contextual: Contextual = [X] => (ord: Ordering[X], x$1: X) ?=> ord
    val annotated: Annotated = [X] => (ord: Ordering[X]) ?=> (i: Int) => i + 1
    val operator: Operator = [X] => (ord: Ordering[X]) ?=> (i: Int) => i + 2
    val term = [X: Ordering as ord] => (x: ord.type) => x eq ord
    val emptyLiteral = [X: Ordering] => () => summon[Ordering[X]].hashCode * 0 + 7
    val emptyAsContext: [X] => Ordering[X] ?=> Int = emptyLiteral
    val emptyType: EmptyType = [X] => () => (o: Ordering[X]) ?=> 3
    val synthetic: Synthetic = [X] => (i: Int) => i
    given ordS: Ordering[String] = Ordering.String
    println(singleton[String](ordS))
    println(contextual[String](using ordS, "a") eq ordS)
    println(annotated[String](1))
    println(operator[String](1))
    println(emptyAsContext[String])
    println(term[String](using ordS)(ordS))
    println(emptyType[String]())
    println(synthetic[String](4))
    println(local(5))
    println(literal(6)[String]("seven"))
    println(kinded[Int, List](List(8)))
    val siblings: Siblings[Int, String] = (9, "ten")
    println(siblings)
  }
}
