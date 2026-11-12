// A class's `@threadUnsafe` lazy val whose initialiser reads the lazy val itself once, on one thread. dotty's
// transformMemberDefThreadUnsafe keeps no state for a value being computed (the flag is set after the right-hand
// side), so the inner read runs the initialiser again; a thread-safe lazy val would wait on itself here.
import scala.annotation.threadUnsafe

class Fold:
  var depth = 0
  @threadUnsafe lazy val x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }

@main def run(): Unit =
  val f = new Fold
  println(s"x = ${f.x} after ${f.depth} run(s)")
  println(s"x = ${f.x} after ${f.depth} run(s)")
