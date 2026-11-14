// A library whose given class is typed through the library's own alias of a val's singleton type, the val a
// member of an object compiled with the library that a program over the jar defines again
// (tests/pending-cases/jar_alias_names_program_val.scala).
package aliasval

trait Show[T]:
  def show: String

object Shelf:
  val size = 3

object Stock:
  val alias = Shelf

object Counter:
  type Held = Stock.alias.type
  given intShow: Show[Int] with
    def show: String = "int"
  given heldShow: Show[Held] with
    def show: String = "held"
