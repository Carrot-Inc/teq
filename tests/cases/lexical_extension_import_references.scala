// An import alternative is a reference: its overloads resolved against the call, where an
// overload the arguments do not decide between fails that import (scalac `B`, where counting it
// made `B` and `A` an ambiguity); an extension two objects inherit from one trait is two
// alternatives, one per object, their ambiguity leaving the selection to the companion (scalac
// `companion`). An explicit `using` list ends the prefix after the receiver, so a later clause
// that depends on the given supplied is the application's (scalac `lexical`). The calls below:
// overloads the argument does not decide, a nullary alternative, an extension two objects
// inherit, and a clause that depends on the given supplied.
class R
object R:
  extension (r: R) def pick(x: Boolean): String = "companion pick"
  extension (r: R) def same(x: Int): String = "companion same"
  extension (r: R) def dep(using k: Key)(x: Int): String = "companion dep"
object A:
  extension (r: R) def pick(x: Int): String = "A int"
  extension (r: R) def pick(x: String): String = "A string"
  extension (r: R) def nul(x: Int): String = "A nul int"
  extension (r: R) def nul(x: String): String = "A nul string"
object B:
  extension (r: R) def pick(x: Boolean): String = "B pick"
  extension (r: R) def nul: String = "B nul"
trait Ops:
  def label: String
  extension (r: R) def same(x: Int): String = label
object C extends Ops:
  def label: String = "C"
object D extends Ops:
  def label: String = "D"
trait Key:
  type V
trait Evidence[A]:
  def show: String
object Evidence:
  given Evidence[Int] with
    def show: String = "lexical"
object Main:
  extension (r: R) def dep(using k: Key)(using e: Evidence[k.V])(x: Int): String = e.show
  def main(args: Array[String]): Unit =
    locally {
      import A.*
      import B.*
      println((new R).pick(true))
      println((new R).nul)
    }
    locally {
      import C.*
      import D.*
      println((new R).same(1))
    }
    val key: Key { type V = Int } = new Key { type V = Int }
    println((new R).dep(using key)(1))
