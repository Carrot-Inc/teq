// A quote's copy is named by its site's file, which a build knows by the file's identity: the
// shortest suffix of its path that no other file of the build shares (`Use.scala` for
// `b/Use.scala` alone, `b/Use.scala` beside `a/Use.scala`), a function of the set of inputs
// and not of their order (tests/split.sh builds the three files in two orders and the two
// without `a/Use.scala`).
package keys

import scala.quoted.*

trait Show:
  def show: String

object Macros:
  inline def make(inline s: String): Show = ${ impl('s) }

  def impl(s: Expr[String])(using Quotes): Expr[Show] =
    '{ new Show { def show: String = $s } }
