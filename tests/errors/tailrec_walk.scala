// expect: 15:16: error: Cannot rewrite recursive call: it is not in tail position
// expect: 16:16: error: TailRec optimisation not applicable, method local contains no recursive calls
// expect: 19:16: error: TailRec optimisation not applicable, method anon contains no recursive calls
// expect: 20:16: error: Cannot rewrite recursive call: it is not in tail position
// expect: 21:16: error: TailRec optimisation not applicable, method dropped contains no recursive calls
// expect: 5 errors found
// The calls `@tailrec` counts are those the expanded body reaches (`TailRec.transformDefDef`): a lambda's, in no
// tail position; not a local def's nor a class's the body makes (scalac warns of the local def's call, E199); an
// inline argument once per copy the expansion makes, none where it drops it.
import scala.annotation.tailrec
inline def twice(inline a: Int): Int = a + a
inline def drop(inline a: Int): Int = 0
trait F { def get: Int }
object O:
  @tailrec def lam(n: Int): Int = if n == 0 then 0 else List(1).map(x => lam(n - 1)).head
  @tailrec def local(n: Int): Int =
    def inner(k: Int): Int = local(k)
    if n == 0 then 0 else inner(n - 1)
  @tailrec def anon(n: Int): Int = if n == 0 then 0 else new F { def get = anon(n - 1) }.get
  @tailrec def tw(n: Int): Int = if n == 0 then 0 else twice(tw(n - 1))
  @tailrec def dropped(n: Int): Int = drop(dropped(n - 1))
