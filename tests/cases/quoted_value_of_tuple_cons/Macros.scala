// `Type.valueOfTuple` over a tuple type that is a chain of `*:`: a tuple of more than 22 elements has no other
// type, and a short chain may be written so.
import scala.quoted.*

object TupleValues:
  inline def long: String = ${ longImpl }
  inline def chain: String = ${ chainImpl }

  def longImpl(using Quotes): Expr[String] =
    Expr(Type.valueOfTuple[(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)].toString)

  def chainImpl(using Quotes): Expr[String] =
    Expr(Type.valueOfTuple[1 *: "b" *: EmptyTuple].toString)
