// A trait whose self type names a class that extends it (tapir's `EndpointTransputMacros[T]
// { this: EndpointTransput[T] => }`): the self type resolves after the parents, its type members
// are in scope by name and through `C.this`, an inline method of the trait expands on a stable, an
// unstable and a widened receiver, and a `Mirror.ProductOf[T]` proxy keeps its refinement.
import scala.deriving.Mirror

trait Macros[T]:
  this: Transput[T] =>
  inline def mapTo[C <: Product](using mc: Mirror.ProductOf[C]): ThisType[C] =
    this.map(t => mc.fromProduct(t.asInstanceOf[Tuple]))

sealed trait Transput[T] extends Macros[T]:
  type ThisType[X] <: Transput[X]
  def map[U](f: T => U): ThisType[U]
  def show: String

case class Input[T](name: String, value: T) extends Transput[T]:
  type ThisType[X] = Input[X]
  def map[U](f: T => U): Input[U] = Input(name + ".mapped", f(value))
  def show: String = name + "=" + value

case class Pair(id: Int, name: String)

object Main:
  def pick(b: Boolean): Transput[(Int, String)] = if b then Input("a", (1, "x")) else Input("b", (2, "y"))
  def main(args: Array[String]): Unit =
    println(pick(true).mapTo[Pair].show)
    val m = summon[Mirror.ProductOf[(Int, String)]]
    val t: (Int, String) = m.fromProduct((3, "z"))
    println(t)
    println(pick(false).mapTo[(Int, String)].show)
    val widened: Transput[(Int, String)] = Input("w", (4, "q"))
    println(widened.mapTo[(Int, String)].show)
