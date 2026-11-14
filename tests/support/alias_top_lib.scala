// A library whose given class is typed through an alias at the top level of the library's package, the alias
// of a val's singleton type, the val a member of an object compiled with the library that a program over the
// jar defines again (tests/pending-cases/jar_toplevel_alias_names_program_val.scala).
package aliastop

trait Show[T]:
  def show: String

object Shelf:
  val size = 3

object Stock:
  val alias = Shelf

type Held = Stock.alias.type

object Counter:
  given intShow: Show[Int] with
    def show: String = "int"
  given heldShow: Show[Held] with
    def show: String = "held"
