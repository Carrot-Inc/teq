// A kept call's site context, retained until the inlining phase expands it: a given of the enclosing
// block, an import of the class body, and a block inside an inline argument, whose given its own call
// finds (dotty's phase walks with the tree's own context, `TreeMapWithPreciseStatContexts`).
import scala.compiletime.summonInline
class Token(val n: Int)
object Tokens { given Token = new Token(22) }
inline def get: Int = summonInline[Token].n
inline def ident(inline x: Int): Int = x
class C:
  import Tokens.given
  val n: Int = get
@main def run(): Unit =
  val a = { given Token = new Token(11); get }
  val b = ident { given Token = new Token(33); get }
  println(a)
  println(new C().n)
  println(b)
