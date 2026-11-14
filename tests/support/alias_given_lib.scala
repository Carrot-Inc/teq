// A library of three givens whose signature names the library's own alias of a val's singleton type, the val a
// member of an object compiled with the library that a program over the jar defines again: in the using clause
// of a given class (aliasusing), in the using clause of an alias given (aliasmark), in a refined result
// (aliasrefined). The programs: tests/errors/jar_alias_in_using_names_program_val.scala,
// jar_alias_in_alias_given_using_names_program_val.scala, jar_alias_in_refined_result_names_program_val.scala.
package aliasusing:
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
    given heldShow(using Show[Held]): Show[Int] with
      def show: String = "held"

package aliasmark:
  trait Show[T]:
    def show: String

  trait Mark[T]

  object Shelf:
    val size = 3

  object Stock:
    val alias = Shelf

  object Counter:
    type Held = Stock.alias.type
    given intShow: Show[Int] with
      def show: String = "int"
    given heldShow(using x: Mark[Held]): Show[Int] = new Show[Int]:
      def show: String = "held"

package aliasrefined:
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
    trait R:
      type T
      def show: String
    given heldShow: (R { type T = Held }) = new R:
      type T = Held
      def show: String = "held"
