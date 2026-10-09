// The jar side of tests/cases/view_lib_alias and view_lib_quotes, compiled by scalac: what the
// loader's lock holder types from TASTy while a worker already holds its own
// equivalent.
package viewlib

import scala.quoted.*

// A match-type alias of a trait that names the trait's parameter, seen through an object: a
// member lookup copies it once per prefix, which a worker's body and a library body both ask for.
trait Kind[T]:
  type Lift[X] = X match
    case Int => T
    case _ => X

object K extends Kind[String]

object Use:
  inline def lifted: K.Lift[Int] = "lifted"

final case class Box[A](a: A)

object Lib:
  inline def boxedInt(inline a: Int): Box[Int] = Box(a)
  inline def ident[T](inline t: T): T = t

  // A quote with a deferred inline call whose signature's result is `Box[Int]`.
  inline def make(inline x: Int): Box[Int] = ${ makeImpl('x) }
  def makeImpl(x: Expr[Int])(using Quotes): Expr[Box[Int]] = '{ boxedInt($x + 1) }

  // A quote with an inline call whose type argument is `Box[Int]`.
  inline def keep(inline b: Box[Int]): Box[Int] = ${ keepImpl('b) }
  def keepImpl(b: Expr[Box[Int]])(using Quotes): Expr[Box[Int]] = '{ ident[Box[Int]]($b) }
