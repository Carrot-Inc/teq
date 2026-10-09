// jars: scala-library chimney-jvm chimney-macro-commons-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep io.scalaland::chimney:1.11.0
// chimney's `withFallback` in link mode: the derivation reads the fallback's getters through
// `Existential[Getter[fallback.Underlying, *]]`, a type lambda over a binder of the macro's
// body, which the loader keeps in source form.
import io.scalaland.chimney.dsl.*

final case class Inner(a: Int, b: String)
final case class Outer(inner: Inner, c: Double)
final case class Flat(a: Int, b: String, c: Double)

def flatten(o: Outer): Flat = o.into[Flat].withFallback(o.inner).transform
def viaVal(o: Outer): Flat =
  val t = o.into[Flat].withFallback(o.inner)
  t.transform

@main def run(): Unit =
  println(flatten(Outer(Inner(1, "x"), 2.0)))
  println(viaVal(Outer(Inner(3, "y"), 4.5)))
