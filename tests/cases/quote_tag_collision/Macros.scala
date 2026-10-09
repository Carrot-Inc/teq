// The two files of the sites have names whose tags (the FNV-1a hash of a file's key, base 36)
// are the same 32 bits: their copies of the quote's class must still be two classes.
// teq: --no-outline
package collision

import scala.quoted.*

trait Show:
  def show: String

object Macros:
  inline def make(inline s: String): Show = ${ impl('s) }

  def impl(s: Expr[String])(using Quotes): Expr[Show] =
    '{ new Show { def show: String = $s } }
