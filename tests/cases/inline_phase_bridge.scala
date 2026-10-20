// A plain call inside a plain inline body expands where the walk reaches it, the body's own given in
// scope at the site (`summonInline` of the expanded body's block).
import scala.compiletime.summonInline
class Token(val n: Int)
inline def get: Int = summonInline[Token].n
inline def wrap: Int = { given Token = new Token(44); get }
@main def run(): Unit = println(wrap)
