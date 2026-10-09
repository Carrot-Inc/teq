import scala.quoted.*

// A macro run nested at the same outermost site as another (`inner()`, quoted by `outer()`)
// numbers its fresh names after the outer run's: two calls of `freshName("v")` under one site give
// two names; scalac numbers them per compilation unit.
object Fresh:
  inline def inner(): String = ${ innerImpl() }
  def innerImpl()(using q: Quotes): Expr[String] =
    import q.reflect.*
    Expr(Symbol.freshName("v"))
  inline def outer(): Boolean = ${ outerImpl() }
  def outerImpl()(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    val a = Symbol.freshName("v")
    '{ ${ Expr(a) } == inner() }
  inline def two(): Boolean = ${ twoImpl() }
  def twoImpl()(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    Expr(Symbol.freshName("w") == Symbol.freshName("w"))
