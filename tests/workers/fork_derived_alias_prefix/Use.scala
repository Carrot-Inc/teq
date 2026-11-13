// A match alias of a generic class seen through a prefix written as a type (`Kind[String]#Elem`): the alias's
// copy is kept under the prefix's type. `Elem[Int]` reduces to `String`; scalac 3.8.4 prints `a`.
class Kind[T] { type Elem[X] = X match { case Int => T; case _ => X } }
object Use {
  def f(): String = {
    val e: Kind[String]#Elem[Int] = "a"
    e
  }
}
@main def main(): Unit = println(Use.f())
