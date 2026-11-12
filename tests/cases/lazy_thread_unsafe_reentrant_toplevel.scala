// A file's top-level `@threadUnsafe` lazy val whose initialiser reads the lazy val itself once
// (lazy_thread_unsafe_reentrant): a field of the file's module.
import scala.annotation.threadUnsafe

var depth = 0
@threadUnsafe lazy val x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }

@main def run(): Unit =
  println(s"x = $x after $depth run(s)")
  println(s"x = $x after $depth run(s)")
