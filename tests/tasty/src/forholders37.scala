package fix.forh37

import scala.language.`3.7`

// Under source 3.7, `makeFor`'s better fors are off (`SourceVersion.enablesBetterFors`), so a generator's aliases
// before the end go the fallback's way: the generator's and the alias's `makeIdPat` names, then the alias's holder
// (the pass's probe on 566d48857, For37.scala, read under the unit's version from its import here).
object For37 {
  def f = for {
    (x, y) <- List((1, 2))
    (a, b) = (3, 4)
  } yield x + y + a + b
}
object After37 {
  val List(a, b) = List(1, 2): @unchecked
}
