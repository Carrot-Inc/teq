// An extension the companion object inherits from a trait is the implicit scope's reference to that object, and the
// view search's ambiguity is the qualifier's mismatch (Typer.scala:4346-4353, `err.typeMismatch(qual, selProto,
// notes)`): scalac 3.8.4 names `object Foo`, not the trait's method, at the qualifier. teq reports its own ambiguity at
// the selection, the companion by the trait: master (efddb38de3) and the head (61c1af7941) "both FooOps and
// given_Ops_Foo provide show on Foo". Queued: push 6 (the companion references in the one ranked list, then
// typeMismatch whole).
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
