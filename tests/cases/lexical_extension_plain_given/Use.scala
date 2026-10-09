// A plain inline given that an extension's prefix resolves is registered for the later phase
// with the candidate, not expanded: the lexical candidate that fails on `Need` drops it, the
// companion's candidate resolves it again, and its macro runs once, where scalac's `Inlining`
// phase runs it (scalac `1`, `2`; an expansion for both candidates would print `2`, `3`).
// The given of a candidate selected expands once, before the
// argument's call, as its place in the tree orders them (`33`, then `5`).
trait Need
class R
object R:
  extension (r: R)(using x: X) def pick: Int = x.n
class S
object S:
  extension (s: S) def take(k: Int): Int = -k
object Main:
  extension (r: R)(using X, Need) def pick: Int = -1
  extension (s: S)(using x: X) def take(k: Int): Int = x.n * 10 + k
  def main(args: Array[String]): Unit =
    println((new R).pick)
    println(M.next)
    println((new S).take(M.next - 1))
    println(M.next)
