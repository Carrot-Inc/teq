package scala.reflect

/** A `TypeTest[S, T]` tells at run time whether a value of type `S` is a `T`, as scala-library
  * 3.8.4's `scala/reflect/TypeTest.scala` declares it: `unapply` is the extractor that a type
  * pattern `case t: T` over an abstract `T` goes through where an instance is in scope
  * (`typer/Typer.scala` 1382 to 1404, `tryWithTypeTest`; `Typer::tag_pattern`). The compiler
  * synthesizes one as dotty's second special handler does (`typer/Synthesizer.scala` 95 to 126,
  * `Typer::type_test_given`): `identity` where `S` conforms to `T`, otherwise an instance whose
  * `unapply` is the `isInstanceOf[T]` test. On the JVM the trait is scala-library's own
  * interface, `unapply(Object): Option` once erased.
  */
@implicitNotFound("No TypeTest available for [${S}, ${T}]")
trait TypeTest[-S, T] extends Serializable:
  def unapply(x: S): Option[x.type & T]

object TypeTest:
  /** The test that always succeeds, `null` included. */
  def identity[T]: TypeTest[T, T] = new TypeTest[T, T]:
    def unapply(x: T): Option[x.type & T] = Some(x)

/** scala-library's `scala.reflect.Typeable`: a `TypeTest` over any value. */
type Typeable[T] = TypeTest[Any, T]
