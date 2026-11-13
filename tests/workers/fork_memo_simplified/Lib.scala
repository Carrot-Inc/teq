// A union the typer simplifies while the workers type the bodies (`Dog | Animal` is `Animal`), read again
// where the file a macro of the run suspended (Use.scala) is typed after the merge: the member is found on
// the simplified type. scalac 3.8.4 prints `dog1`.
import scala.quoted.*
class Animal { def name = "animal" }
class Dog extends Animal { override def name = "dog" }
class Box[+T](val get: T)
object Lib {
  def mk(): Box[Dog | Animal] = new Box(new Dog)
  inline def m(inline x: Int): Int = ${ impl('x) }
  def impl(x: Expr[Int])(using Quotes): Expr[Int] = x
}
