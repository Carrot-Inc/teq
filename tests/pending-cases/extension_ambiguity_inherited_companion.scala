// An extension the companion object inherits from a trait is the implicit
// scope's reference to that object, and the view search's ambiguity is
// the qualifier's mismatch (Typer.scala:4346-4353, `err.typeMismatch(qual,
// selProto, notes)`): scalac 3.8.4 names `object Foo`, not the
// trait's method, at the qualifier. Queued: push 6 (the companion
// references in the one ranked list, then typeMismatch whole).
trait Ops[A]:
  extension (a: A) def show: String
trait FooOps:
  extension (f: Foo) def show = "inherited companion"
class Foo
object Foo extends FooOps:
  given Ops[Foo] with
    extension (a: Foo) def show = "given"
object Main:
  def main(args: Array[String]): Unit = println(Foo().show)
