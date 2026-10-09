package mgh

import scala.quoted.*

// Run by the interpreter while the downstream module is typed: `DatabaseUtils` is initialised,
// the bodies of its givens typed from the upstream module's TASTy.
object Mac:
  inline def touch: String = ${ touchImpl }
  def touchImpl(using Quotes): Expr[String] =
    Expr(if mgg.DatabaseUtils.toString.nonEmpty then "touched" else "none")
