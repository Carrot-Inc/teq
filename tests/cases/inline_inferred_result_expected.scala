// An inline method declared without a result type, given where a type of the call it is an argument of is expected
// (indigo's `NoOp.prepare[E]` for a `Shader[Env, Unit]` of ultraviolet's opaque `Shader`): its type arguments are
// constrained by the result its body infers, as dotty's `Namer.inferredResultType` gives it before any call, a call
// placed before the definition too (`StandardShaders` before `NoOp`).
object shaders:
  opaque type Shader[In, Out] = In => Out
  object Shader:
    inline def apply[In](f: In => Unit): Shader[In, Unit] = f
    extension [In, Out](s: Shader[In, Out]) def run(in: In): Out = s(in)
import shaders.Shader

object Standard:
  lazy val first: String = Use.fragment(Blit.fragment, NoOp.prepare, Blit.Env(3))

object NoOp:
  inline def prepare[E] = Shader[E] { _ => () }
object Blit:
  final case class Env(x: Int)
  inline def fragment = Shader[Env] { env => println(env.x) }
object Use:
  inline def fragment[Env](inline a: Shader[Env, Unit], inline b: Shader[Env, Unit], env: Env): String =
    a.run(env); b.run(env); "two"
  inline def fragment[Env](inline a: Shader[Env, Unit], env: Env): String = "one"

@main def run(): Unit =
  println(Use.fragment(Blit.fragment, NoOp.prepare, Blit.Env(1)))
  println(Use.fragment(NoOp.prepare, Blit.Env(2)))
  println(Standard.first)
