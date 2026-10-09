// expect: No given instance of type Int was found.
// A given an expanded body defines in a block is in scope in that block alone: the second `get`
// finds none where no caller's given stands, as scalac 3.8.4 rejects it
// (tests/cases/inline_summon_body_givens finds the caller's).
import scala.compiletime.summonInline
inline def get: Int = summonInline[Int]
inline def outer: Int = {
  { given Int = 7; get }
  get
}
@main def run(): Unit = println(outer)
