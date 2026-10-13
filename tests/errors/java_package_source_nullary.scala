// A program's class in a `java.*` package is Scala's: `matchNullaryLoosely` holds of a Java-defined
// member, `Any`'s, `Object.clone` or a Scala 2 class's, by where it comes from and never by its
// package's name, so a parameterless member takes no `()` (E050) and a `def f()` is no `f` (E100).
// Master took both as a JDK class's.
package java.example
class C { def f: Int = 1; def g(): Int = 2 }
val x = (new C).f()
val y = (new C).g
// expect: 7:9: error: method f in class C does not take parameters
// expect: 8:9: error: method g in class C must be called with () argument
