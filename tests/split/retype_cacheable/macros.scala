package cacheable

import scala.quoted.*

object Token

enum Kind(val n: Int):
  case A extends Kind(1)
  case B extends Kind(2)

/** An object declared to hold cacheable state: kept across the retypes of a session, with what it
  * reaches. `uses` counts its expansions, which a kept object carries on where a fresh build
  * starts again at one: the declaration broken on purpose, so that keeping shows, in the output and
  * in a warning for a session that only types. */
object Cache:
  var uses: Int = 0
  val token = Token
  val kind = Kind.A
  /** A cycle of the object's own: a closure of its field over `this`, which the registry of a
    * resident holds (`src/interp/registry.rs`). */
  val counted: () => Int = () => uses

/** An object with cacheable state that holds a tree: dropped at the next build's start,
  * with one warning, and made anew. */
object Holder:
  var tree: Any = null

object Outer:
  object Inner:
    var uses: Int = 0

/** A cache half made when the expansion ran out of its budget: the caches go with the run. */
object Half:
  val entries = scala.collection.mutable.ListBuffer[Int]()

object Macros:
  def usesImpl(using Quotes): Expr[Int] =
    Cache.uses += 1
    quotes.reflect.report.warning("uses " + Cache.uses + " " + Cache.counted())
    Expr(Cache.uses)

  def sameImpl(using Quotes): Expr[Boolean] =
    val all = Kind.values
    val same = (Cache.token eq Token) && (Cache.kind eq Kind.A) && (Cache.kind eq all(0))
    quotes.reflect.report.warning("same " + same)
    Expr(same)

  def heldImpl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    if Holder.tree == null then Holder.tree = Literal(IntConstant(7))
    Holder.tree.asInstanceOf[Term].asExprOf[Int]

  def innerImpl(using Quotes): Expr[Int] =
    Outer.Inner.uses += 1
    Expr(Outer.Inner.uses)

  def halfImpl(using Quotes): Expr[Int] =
    Half.entries += 1
    Expr(deeper(0))

  def deeper(n: Int): Int = deeper(n + 1) + 1

  def halfMadeImpl(using Quotes): Expr[Int] =
    quotes.reflect.report.error("half made " + Half.entries.size)
    Expr(0)


inline def uses: Int = ${ Macros.usesImpl }
inline def same: Boolean = ${ Macros.sameImpl }
inline def held: Int = ${ Macros.heldImpl }
inline def inner: Int = ${ Macros.innerImpl }
inline def half: Int = ${ Macros.halfImpl }
inline def halfMade: Int = ${ Macros.halfMadeImpl }
