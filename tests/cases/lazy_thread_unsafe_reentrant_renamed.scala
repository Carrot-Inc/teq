// `@threadUnsafe` under other names (lazy_thread_unsafe_reentrant): a renaming import, a type alias and the
// qualified name mark a lazy val as the annotation's class does (dotty's SymUtils.isThreadUnsafe), and a program's
// own annotation named threadUnsafe does not: its lazy val is thread-safe, read here without re-entry.
import scala.annotation.{threadUnsafe as racy}

type Unsynchronised = scala.annotation.threadUnsafe

object own:
  class threadUnsafe extends scala.annotation.StaticAnnotation

class Fold:
  var a = 0
  @racy lazy val renamed: Int = { a += 1; if a > 1 then 1 else renamed + 1 }
  var b = 0
  @Unsynchronised lazy val aliased: Int = { b += 1; if b > 1 then 1 else aliased + 1 }
  var c = 0
  @scala.annotation.threadUnsafe lazy val qualified: Int = { c += 1; if c > 1 then 1 else qualified + 1 }
  var d = 0
  @own.threadUnsafe lazy val other: Int = { d += 1; 7 }

@main def run(): Unit =
  val f = new Fold
  println(s"renamed = ${f.renamed} after ${f.a} run(s)")
  println(s"aliased = ${f.aliased} after ${f.b} run(s)")
  println(s"qualified = ${f.qualified} after ${f.c} run(s)")
  println(s"the program's own annotation: ${f.other} ${f.other} after ${f.d} run(s)")
