// A trait's `@threadUnsafe` lazy val held by a class and by an object, its initialiser reading the lazy val itself
// once (lazy_thread_unsafe_reentrant): Mixin copies the member's annotations to the class's copy
// (MixinOps.mkForwarderSym), so the class's lazy val is unsynchronised too.
import scala.annotation.threadUnsafe

trait Fold:
  var depth = 0
  @threadUnsafe lazy val x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }

class InClass extends Fold
object InObject extends Fold

@main def run(): Unit =
  val c = new InClass
  println(s"in a class: x = ${c.x} after ${c.depth} run(s)")
  println(s"in an object: x = ${InObject.x} after ${InObject.depth} run(s)")
  println(s"again: ${c.x} ${InObject.x} after ${c.depth} and ${InObject.depth} run(s)")
