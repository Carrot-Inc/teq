// lila: Puzzle.complete = OpenBody: onComplete(form)(id, angle, mobileBc = false)
// with OpenBody(f: BodyContext[?] ?=> Fu[Result]) and onComplete[A](...)(using BodyContext[A]).
// The given `ctx: BodyContext[?]` is a stable path, so scalac capture-converts its wildcard
// (TypeComparer.compareCaptured: BodyContext[?] as ctx.type <: BodyContext[ctx.A]) and
// instantiates A := ctx.A.
final class BodyContext[A](val body: A)

def onComplete[A](id: Int)(using ctx: BodyContext[A]): String = s"complete $id with ${ctx.body}"
def openBody(f: BodyContext[?] ?=> String): String = f(using BodyContext("payload"))

@main def run() =
  println(openBody(onComplete(1)))
