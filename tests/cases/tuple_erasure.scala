// The tuple supertypes erase as scalac's do, `Tuple`, `NonEmptyTuple` and a `*:` of no known
// arity to `Product`: no tuple class at run time extends them, so a cast to their own classes
// fails.
import scala.deriving.Mirror

case class Name(schema: String, name: String)

inline def fromArray[A <: Product](using m: Mirror.ProductOf[A]): Array[Any] => A =
  a => m.fromProduct(Tuple.fromArray(a))

def widen(a: Array[Any]): Tuple = Tuple.fromArray(a)
def nonEmpty(t: Tuple): NonEmptyTuple = t.asInstanceOf[NonEmptyTuple]
def cons(t: Any): Int *: Tuple = t.asInstanceOf[Int *: Tuple]

object Main:
  def main(args: Array[String]): Unit =
    println(fromArray[Name](Array("public", "users")))
    println(widen(Array(1, 2)))
    println(nonEmpty((1, "a")))
    println(cons((3, "b", true)))
