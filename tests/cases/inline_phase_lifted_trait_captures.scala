// A local trait whose member reaches a local given only through a plain inline call's expansion
// (`summonInline`, searched where the inlining phase expands it) takes the given as a proxy after
// the phase, and the classes mixing it in pass it, as dotty's `LambdaLift` lifts a local trait
// (`Dependencies.isLocal`) after `Inlining`: a member, a lazy val, a trait extending it, a trait's
// initialiser, an anonymous class and a call from another local class (scalac prints 42 and 112).
import scala.compiletime.summonInline

trait TC:
  def get: Int

inline def summonTC: Int = summonInline[TC].get

def f(n: Int): Int =
  given TC with
    def get = n
  trait Local:
    def value: Int = summonTC
  class Child extends Local
  Child().value

def g(n: Int): Int =
  given TC with
    def get = n
  trait Local:
    def value: Int = summonTC
    lazy val lz: Int = summonTC + 1
  trait Local2 extends Local:
    def twice: Int = value * 2
  trait WithInit:
    val init: Int = summonTC + 100
  class Child extends Local2
  class Both extends Local with WithInit
  class User:
    def use(l: Local): Int = l.value + 3
  val anon = new Local {}
  Child().value + Child().twice + Child().lz + Both().init + User().use(anon) + anon.lz

@main def run(): Unit =
  println(f(42))
  println(g(1))
