// jars: javareview-lib
// jdk: none
// expect: 7:32: error: method f in class C does not take parameters
// A Scala 3 jar's parameterless member of a `java.*` package is Scala's with no JDK to ask as with one
// (`matchNullaryLoosely` by the declaration's provenance, never the package's name): `f()` is refused,
// as scalac refuses it (E050).
def f(c: java.review.C): Int = c.f()
